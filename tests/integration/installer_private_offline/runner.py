#!/usr/bin/env python3
"""Bounded, network-isolated actual-script tests; bwrap is mandatory."""
import hashlib
import json
import os
from pathlib import Path
import resource
import select
import shutil
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / 'scripts/install/hypermesh-install.sh'
OLD = b'prior installed binary\n'
PAYLOAD = b'#!/bin/sh\nprintf executed > "$HOME/sentinel"\n'
DIGEST = hashlib.sha256(PAYLOAD).hexdigest()
ALLOWED = 'mkdir cp chmod mv rm mktemp sha256sum tr stat id dirname'.split()
BLOCKED = 'cargo rustup curl wget sudo systemctl service openssl hypermesh ip ssh'.split()


def write(path, data, mode=0o600):
    path.write_bytes(data if isinstance(data, bytes) else data.encode())
    path.chmod(mode)


def limits():
    resource.setrlimit(resource.RLIMIT_FSIZE, (10 * 1024 * 1024,) * 2)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def wrappers(case, fault):
    bindir = case / 'commands'
    bindir.mkdir()
    for name in ALLOWED:
        (bindir / name).symlink_to(shutil.which(name))
    for name in BLOCKED:
        write(bindir / name, '#!/bin/sh\nexit 97\n', 0o700)
    if fault not in ('copy', 'chmod', 'rename', 'interrupt', 'write', 'mutate',
                     'cleanup-stage', 'cleanup-backup', 'postcommit-interrupt',
                     'directory-substitution', 'unsupported-mv'):
        return bindir
    name = {'copy': 'cp', 'interrupt': 'cp', 'mutate': 'cp',
            'rename': 'mv', 'write': 'mktemp', 'chmod': 'chmod',
            'cleanup-stage': 'rm', 'cleanup-backup': 'rm',
            'postcommit-interrupt': 'mv', 'directory-substitution': 'mv',
            'unsupported-mv': 'mv'}[fault]
    (bindir / name).unlink()
    real = shutil.which(name)
    body = f'#!/bin/sh\nreal={real}\n'
    if fault in ('copy', 'write'):
        body += 'exit 73\n'
    elif fault == 'unsupported-mv':
        body += 'exit 73\n'
    elif fault == 'directory-substitution':
        body += 'case "$*" in */new*) printf %s "$PPID" > "$HOME/ready"; read token < "$HOME/barrier";; esac\nexec "$real" "$@"\n'
    elif fault == 'chmod':
        body += 'case "$*" in *0755*) exit 73;; esac\nexec "$real" "$@"\n'
    elif fault == 'rename':
        body += 'case "$*" in */new*) exit 73;; esac\nexec "$real" "$@"\n'
    elif fault.startswith('cleanup-'):
        pattern = '.hypermesh-install.' if fault == 'cleanup-stage' else '.hypermesh-previous.'
        body += f'case "$*" in *{pattern}*) test -f "$HOME/failed" || {{ : > "$HOME/failed"; exit 73; }};; esac\n'
        body += 'exec "$real" "$@"\n'
    elif fault == 'postcommit-interrupt':
        body += '"$real" "$@" || exit\ncase "$*" in */new*) printf %s "$PPID" > "$HOME/ready"; read token < "$HOME/barrier";; esac\n'
    elif fault == 'mutate':
        body += '"$real" "$@" || exit\nfor last do :; done\nprintf tampered >> "$last"\n'
    else:
        body += '"$real" "$@" || exit\nprintf %s "$PPID" > "$HOME/ready"\nread token < "$HOME/barrier"\n'
    write(bindir / name, body, 0o700)
    return bindir


def fixture(base, name, fault):
    case = base / name
    case.mkdir(mode=0o700)
    for part in ('bin', 'data', 'home', 'tmp'):
        (case / part).mkdir()
    write(case / 'bin/hypermesh', OLD, 0o751)
    write(case / 'artifact', PAYLOAD, 0o700)
    for asset in ('identity', 'assets', 'config'):
        write(case / 'data' / asset, f'preserve {asset}\n')
    commands = wrappers(case, fault)
    env = dict(PATH=str(commands), HOME=str(case / 'home'), TMPDIR=str(case / 'tmp'),
               HYPERMESH_PREBUILT_BINARY=str(case / 'artifact'),
               HYPERMESH_PREBUILT_SHA256=DIGEST, HYPERMESH_OFFLINE='1',
               HYPERMESH_NO_START='1', HYPERMESH_INSTALL_DIR=str(case / 'bin'),
               HYPERMESH_DATA_DIR=str(case / 'data'))
    if name in ('valid-colon', 'relative-colon', 'valid-backslash'):
        alias = 'root:colon' if name != 'valid-backslash' else 'root\\backslash'
        (case / alias).mkdir()
        env['HYPERMESH_INSTALL_DIR'] = str(case / alias / '../bin')
    if name == 'valid-colon':
        env['HYPERMESH_DATA_DIR'] = str(case / 'root:colon/../data')
    return case, env


def argv_for(case, env, shell, script):
    argv = ['bwrap', '--unshare-user', '--unshare-net', '--unshare-ipc', '--unshare-uts',
            '--new-session', '--die-with-parent', '--cap-drop', 'ALL',
            '--ro-bind', '/', '/', '--bind', str(case), str(case),
            '--proc', '/proc', '--dev', '/dev', '--clearenv']
    for key, value in env.items():
        argv.extend(['--setenv', key, value])
    return argv + ['--chdir', str(case), shell, str(script)]


def substitute_directory(case):
    destination = case / 'bin/hypermesh'
    destination.rename(case / 'displaced-original')
    destination.mkdir()
    write(destination / 'unrelated', b'preserve unrelated directory\n')
    with (case / 'home/barrier').open('w') as barrier:
        barrier.write('continue\n')


def handle_barrier(case, ready_fd, barrier_kind):
    assert select.select([ready_fd], [], [], 30)[0], 'no staging barrier'
    installer_pid = int(os.read(ready_fd, 32))
    assert os.getpgid(installer_pid) == installer_pid
    if barrier_kind == 'directory-substitution':
        substitute_directory(case)
    else:
        os.killpg(installer_pid, signal.SIGTERM)


def run_process(case, argv, barrier_kind):
    ready_fd = None
    if barrier_kind:
        for name in ('ready', 'barrier'):
            os.mkfifo(case / 'home' / name, 0o600)
        ready_fd = os.open(case / 'home/ready', os.O_RDWR | os.O_NONBLOCK)
    start = time.monotonic()
    out = (case / 'stdout').open('wb')
    err = (case / 'stderr').open('wb')
    proc = subprocess.Popen(argv, stdout=out, stderr=err, start_new_session=True,
                            preexec_fn=limits)
    try:
        if barrier_kind:
            handle_barrier(case, ready_fd, barrier_kind)
        rc = proc.wait(timeout=180)
    except BaseException:
        os.killpg(proc.pid, signal.SIGTERM)
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL)
            proc.wait()
        raise
    finally:
        out.close()
        err.close()
        if ready_fd is not None:
            os.close(ready_fd)
    return rc, start, time.monotonic()


def assert_preserved(case):
    assert (case / 'bin/hypermesh').read_bytes() == OLD, 'old binary changed'
    assert (case / 'bin/hypermesh').stat().st_mode & 0o777 == 0o751
    for asset in ('identity', 'assets', 'config'):
        assert (case / 'data' / asset).read_text() == f'preserve {asset}\n'
    assert sorted(p.name for p in (case / 'data').iterdir()) == ['assets', 'config', 'identity']


def assert_directory_refused(case):
    destination = case / 'bin/hypermesh'
    assert destination.is_dir(), 'unrelated directory removed or replaced'
    assert sorted(p.name for p in destination.iterdir()) == ['unrelated'], 'artifact moved inside directory'
    assert (destination / 'unrelated').read_bytes() == b'preserve unrelated directory\n'
    backups = list((case / 'bin').glob('.hypermesh-previous.*'))
    assert len(backups) == 1 and backups[0].read_bytes() == OLD, 'recovery bytes not retained'
    assert backups[0].stat().st_mode & 0o777 == 0o751
    assert (case / 'displaced-original').read_bytes() == OLD
    assert 'Rollback failed; prior binary retained at' in (case / 'stderr').read_text()
    assert not list((case / 'bin').glob('.hypermesh-install.*')), 'staging leak'


def run_case(base, shell, name, changes=None, fault='', success=False, baseline=False):
    case, env = fixture(base, name, fault)
    for key, value in (changes or {}).items():
        if value is None:
            env.pop(key, None)
        else:
            env[key] = value.replace('$CASE', str(case))
    script = Path(os.environ['BASELINE_SCRIPT']) if baseline else SCRIPT
    argv = argv_for(case, env, shell, script)
    meta = dict(argv=argv, env=env, source=str(script), sha256=hashlib.sha256(script.read_bytes()).hexdigest(),
                mode=oct(script.stat().st_mode & 0o777), utc=time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()))
    barrier_kind = fault if fault in ('interrupt', 'postcommit-interrupt', 'directory-substitution') else ''
    rc, start, end = run_process(case, argv, barrier_kind)
    meta.update(exit=rc, monotonic_start=start, monotonic_end=end)
    write(case / 'result.json', json.dumps(meta, indent=2) + '\n')
    if baseline:
        assert rc == 0 and (case / 'home/sentinel').exists()
        assert (case / 'bin/hypermesh').read_bytes() == PAYLOAD
        return
    assert not (case / 'home/sentinel').exists(), 'artifact executed'
    assert not (case / 'unexpected-relative').exists(), 'relative state path created'
    if success:
        assert rc == 0, (case / 'stderr').read_text()
        assert (case / 'bin/hypermesh').read_bytes() == PAYLOAD
        assert (case / 'bin/hypermesh').stat().st_mode & 0o777 == 0o755
    else:
        assert rc != 0, 'unexpected success'
        if fault == 'directory-substitution':
            assert_directory_refused(case)
        else:
            assert_preserved(case)
    for asset in ('identity', 'assets', 'config'):
        assert (case / 'data' / asset).read_text() == f'preserve {asset}\n'
    if fault != 'directory-substitution':
        assert not list((case / 'bin').glob('.hypermesh-*')), 'staging or backup leak'
    print(f'PASS {shell} {name}', flush=True)


def matrix(base, shell):
    run_case(base, shell, 'valid', success=True)
    run_case(base, shell, 'valid-colon', success=True)
    run_case(base, shell, 'valid-backslash', success=True)
    run_case(base, shell, 'relative-colon', {'HYPERMESH_DATA_DIR': 'unexpected-relative'})
    run_case(base, shell, 'explicit-public-no-start', {'HYPERMESH_OFFLINE': '0', 'HYPERMESH_PRIVACY': 'public'}, success=True)
    run_case(base, shell, 'uppercase', {'HYPERMESH_PREBUILT_SHA256': DIGEST.upper()}, success=True)
    cases = {'missing-hash': {'HYPERMESH_PREBUILT_SHA256': None},
             'online-missing-hash': {'HYPERMESH_PREBUILT_SHA256': None, 'HYPERMESH_OFFLINE': '0'},
             'empty-privacy': {'HYPERMESH_PRIVACY': ''},
             'empty-offline': {'HYPERMESH_OFFLINE': ''},
             'malformed-hash': {'HYPERMESH_PREBUILT_SHA256': 'z' * 64},
             'wrong-hash': {'HYPERMESH_PREBUILT_SHA256': '0' * 64},
             'missing-artifact': {'HYPERMESH_PREBUILT_BINARY': '$CASE/absent'},
             'offline-no-artifact': {'HYPERMESH_PREBUILT_BINARY': None},
             'invalid-privacy': {'HYPERMESH_PRIVACY': 'invalid'},
             'public-offline': {'HYPERMESH_PRIVACY': 'public'},
             'invalid-offline': {'HYPERMESH_OFFLINE': 'yes'},
             'invalid-no-start': {'HYPERMESH_NO_START': 'yes'},
             'offline-start': {'HYPERMESH_NO_START': '0'},
             'implicit-install': {'HYPERMESH_INSTALL_DIR': None},
             'implicit-data': {'HYPERMESH_DATA_DIR': None},
             'bad-data': {'HYPERMESH_DATA_DIR': '$CASE/artifact'},
             'outside-write': {'HYPERMESH_INSTALL_DIR': '/usr/local/installer-test-denied'}}
    for name, changes in cases.items():
        run_case(base, shell, name, changes)
    for fault in ('write', 'copy', 'chmod', 'rename', 'interrupt', 'mutate',
                  'cleanup-stage', 'cleanup-backup', 'postcommit-interrupt',
                  'directory-substitution', 'unsupported-mv'):
        run_case(base, shell, fault, fault=fault)


def main():
    base = Path(sys.argv[1]).resolve()
    assert str(base).startswith('/usr/lib/hypermesh/verification/')
    base.mkdir(mode=0o700, parents=True, exist_ok=False)
    if '--red' in sys.argv:
        run_case(base, '/bin/sh', 'baseline', {'HYPERMESH_PREBUILT_SHA256': '0' * 64}, baseline=True)
        print('RED confirmed: incorrect digest accepted and no-start mock executed')
        return
    shells = [Path('/bin/sh'), Path('/bin/dash'), Path('/bin/bash')]
    for shell in shells:
        if not shell.exists():
            continue
        sub = base / shell.name
        sub.mkdir()
        subprocess.run([str(shell), '-n', str(SCRIPT)], check=True)
        matrix(sub, str(shell))
    total = sum(p.stat().st_size for p in base.rglob('*') if p.is_file() and not p.is_symlink())
    assert total < 100 * 1024 * 1024, 'fixture growth limit'
    print(f'GREEN fixture bytes={total}; no genuine artifact or network operation tested')


if __name__ == '__main__':
    main()
