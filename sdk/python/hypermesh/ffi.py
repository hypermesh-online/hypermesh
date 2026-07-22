"""FFI bindings for libhypermesh_ffi -- native IPC access to HyperMesh daemon.

Loads the ``libhypermesh_ffi`` shared library via ctypes and exposes every
C function declared in ``hypermesh.h`` as a Python method that returns
parsed JSON dicts.

Usage::

    from hypermesh.ffi import HyperMeshFFI

    with HyperMeshFFI() as hm:
        print(hm.status())
        print(hm.dns_list())
        print(hm.caesar_balance())
"""

from __future__ import annotations

import ctypes
import ctypes.util
import json
import os
import platform
from typing import Any, Optional


class FFIError(Exception):
    """Raised when the native library reports an error."""


class LibraryNotFoundError(FFIError):
    """Raised when libhypermesh_ffi cannot be located."""


def _lib_filename() -> str:
    """Return the platform-specific shared library filename."""
    system = platform.system()
    if system == "Darwin":
        return "libhypermesh_ffi.dylib"
    if system == "Windows":
        return "hypermesh_ffi.dll"
    return "libhypermesh_ffi.so"


def _find_library(lib_path: Optional[str] = None) -> str:
    """Locate libhypermesh_ffi using a 4-tier search.

    Search order:
      1. *lib_path* parameter (explicit)
      2. ``HYPERMESH_FFI_LIB`` environment variable
      3. ``ctypes.util.find_library("hypermesh_ffi")``
      4. Common build paths relative to this file
    """
    # 1. Explicit path
    if lib_path:
        if os.path.isfile(lib_path):
            return lib_path
        raise LibraryNotFoundError(
            f"Explicit library path does not exist: {lib_path}"
        )

    # 2. Environment variable
    env_path = os.environ.get("HYPERMESH_FFI_LIB")
    if env_path and os.path.isfile(env_path):
        return env_path

    # 3. System search (LD_LIBRARY_PATH, ldconfig cache, etc.)
    system_path = ctypes.util.find_library("hypermesh_ffi")
    if system_path:
        return system_path

    # 4. Common build paths (relative to repo root)
    lib_name = _lib_filename()
    this_dir = os.path.dirname(os.path.abspath(__file__))
    # sdk/python/hypermesh/ -> repo root is ../../../
    repo_root = os.path.normpath(os.path.join(this_dir, "..", "..", ".."))

    candidates = [
        os.path.join(repo_root, "target", "release", lib_name),
        os.path.join(repo_root, "target", "debug", lib_name),
        os.path.join("/usr", "local", "lib", lib_name),
        os.path.join("/usr", "lib", lib_name),
    ]
    for candidate in candidates:
        if os.path.isfile(candidate):
            return candidate

    searched = "\n  ".join(
        [f"HYPERMESH_FFI_LIB env var", "ctypes.util.find_library"] + candidates
    )
    raise LibraryNotFoundError(
        f"Cannot find {lib_name}. Searched:\n  {searched}\n\n"
        f"Build the FFI crate first:\n"
        f"  cargo build --release -p hypermesh-ffi\n\n"
        f"Or set HYPERMESH_FFI_LIB=/path/to/{lib_name}"
    )


def _load_library(path: str) -> ctypes.CDLL:
    """Load the shared library and bind all function signatures."""
    try:
        lib = ctypes.CDLL(path)
    except OSError as exc:
        raise LibraryNotFoundError(
            f"Failed to load {path}: {exc}"
        ) from exc

    # Opaque handle type
    client_p = ctypes.c_void_p

    # -- Connection lifecycle --
    lib.hypermesh_connect.argtypes = [ctypes.c_char_p]
    lib.hypermesh_connect.restype = client_p

    lib.hypermesh_disconnect.argtypes = [client_p]
    lib.hypermesh_disconnect.restype = None

    # -- Raw RPC --
    lib.hypermesh_call.argtypes = [client_p, ctypes.c_char_p, ctypes.c_char_p]
    lib.hypermesh_call.restype = ctypes.c_void_p

    # -- Node --
    lib.hypermesh_status.argtypes = [client_p]
    lib.hypermesh_status.restype = ctypes.c_void_p

    # -- DNS --
    lib.hypermesh_dns_resolve.argtypes = [client_p, ctypes.c_char_p]
    lib.hypermesh_dns_resolve.restype = ctypes.c_void_p

    lib.hypermesh_dns_list.argtypes = [client_p]
    lib.hypermesh_dns_list.restype = ctypes.c_void_p

    lib.hypermesh_dns_register.argtypes = [
        client_p, ctypes.c_char_p, ctypes.c_char_p,
    ]
    lib.hypermesh_dns_register.restype = ctypes.c_void_p

    # -- Network --
    lib.hypermesh_peers.argtypes = [client_p]
    lib.hypermesh_peers.restype = ctypes.c_void_p

    # -- Blockchain --
    lib.hypermesh_blockchain_height.argtypes = [client_p]
    lib.hypermesh_blockchain_height.restype = ctypes.c_void_p

    lib.hypermesh_blockchain_block.argtypes = [client_p, ctypes.c_uint64]
    lib.hypermesh_blockchain_block.restype = ctypes.c_void_p

    # -- Topology --
    lib.hypermesh_topology_info.argtypes = [client_p]
    lib.hypermesh_topology_info.restype = ctypes.c_void_p

    # -- Assets --
    lib.hypermesh_asset_list.argtypes = [client_p]
    lib.hypermesh_asset_list.restype = ctypes.c_void_p

    lib.hypermesh_asset_store.argtypes = [client_p, ctypes.c_char_p]
    lib.hypermesh_asset_store.restype = ctypes.c_void_p

    lib.hypermesh_asset_fetch.argtypes = [
        client_p, ctypes.c_char_p, ctypes.c_char_p,
    ]
    lib.hypermesh_asset_fetch.restype = ctypes.c_void_p

    # -- Domains --
    lib.hypermesh_domain_list.argtypes = [client_p]
    lib.hypermesh_domain_list.restype = ctypes.c_void_p

    lib.hypermesh_domain_register.argtypes = [
        client_p, ctypes.c_char_p, ctypes.c_char_p,
    ]
    lib.hypermesh_domain_register.restype = ctypes.c_void_p

    # -- Dashboards --
    lib.hypermesh_dashboard_list.argtypes = [client_p]
    lib.hypermesh_dashboard_list.restype = ctypes.c_void_p

    lib.hypermesh_dashboard_deploy.argtypes = [client_p, ctypes.c_char_p]
    lib.hypermesh_dashboard_deploy.restype = ctypes.c_void_p

    # -- Config --
    lib.hypermesh_config_show.argtypes = [client_p]
    lib.hypermesh_config_show.restype = ctypes.c_void_p

    lib.hypermesh_config_get.argtypes = [client_p, ctypes.c_char_p]
    lib.hypermesh_config_get.restype = ctypes.c_void_p

    # -- Caesar EVP --
    lib.hypermesh_caesar_wallet.argtypes = [client_p]
    lib.hypermesh_caesar_wallet.restype = ctypes.c_void_p

    lib.hypermesh_caesar_balance.argtypes = [client_p]
    lib.hypermesh_caesar_balance.restype = ctypes.c_void_p

    lib.hypermesh_caesar_transactions.argtypes = [client_p, ctypes.c_uint32]
    lib.hypermesh_caesar_transactions.restype = ctypes.c_void_p

    lib.hypermesh_caesar_rewards.argtypes = [client_p]
    lib.hypermesh_caesar_rewards.restype = ctypes.c_void_p

    lib.hypermesh_caesar_route_packet.argtypes = [
        client_p, ctypes.c_char_p, ctypes.c_double,
    ]
    lib.hypermesh_caesar_route_packet.restype = ctypes.c_void_p

    lib.hypermesh_caesar_governor_params.argtypes = [client_p]
    lib.hypermesh_caesar_governor_params.restype = ctypes.c_void_p

    # -- TrustChain --
    lib.hypermesh_trustchain_certificates.argtypes = [client_p]
    lib.hypermesh_trustchain_certificates.restype = ctypes.c_void_p

    lib.hypermesh_trustchain_issue.argtypes = [
        client_p, ctypes.c_char_p, ctypes.c_char_p,
    ]
    lib.hypermesh_trustchain_issue.restype = ctypes.c_void_p

    lib.hypermesh_trustchain_validate.argtypes = [client_p, ctypes.c_char_p]
    lib.hypermesh_trustchain_validate.restype = ctypes.c_void_p

    lib.hypermesh_trustchain_revoke.argtypes = [client_p, ctypes.c_char_p]
    lib.hypermesh_trustchain_revoke.restype = ctypes.c_void_p

    lib.hypermesh_trustchain_dns_zones.argtypes = [client_p]
    lib.hypermesh_trustchain_dns_zones.restype = ctypes.c_void_p

    # -- NGauge --
    lib.hypermesh_ngauge_capacity.argtypes = [client_p]
    lib.hypermesh_ngauge_capacity.restype = ctypes.c_void_p

    lib.hypermesh_ngauge_traffic.argtypes = [client_p]
    lib.hypermesh_ngauge_traffic.restype = ctypes.c_void_p

    lib.hypermesh_ngauge_marketplace.argtypes = [client_p]
    lib.hypermesh_ngauge_marketplace.restype = ctypes.c_void_p

    lib.hypermesh_ngauge_node_metrics.argtypes = [client_p]
    lib.hypermesh_ngauge_node_metrics.restype = ctypes.c_void_p

    lib.hypermesh_ngauge_leases.argtypes = [client_p]
    lib.hypermesh_ngauge_leases.restype = ctypes.c_void_p

    # -- Catalog --
    lib.hypermesh_catalog_browse.argtypes = [
        client_p, ctypes.c_char_p, ctypes.c_uint32,
    ]
    lib.hypermesh_catalog_browse.restype = ctypes.c_void_p

    lib.hypermesh_catalog_search.argtypes = [client_p, ctypes.c_char_p]
    lib.hypermesh_catalog_search.restype = ctypes.c_void_p

    lib.hypermesh_catalog_package_info.argtypes = [client_p, ctypes.c_char_p]
    lib.hypermesh_catalog_package_info.restype = ctypes.c_void_p

    lib.hypermesh_catalog_registry_stats.argtypes = [client_p]
    lib.hypermesh_catalog_registry_stats.restype = ctypes.c_void_p

    # -- Memory management --
    lib.hypermesh_free_string.argtypes = [ctypes.c_void_p]
    lib.hypermesh_free_string.restype = None

    lib.hypermesh_last_error.argtypes = [client_p]
    lib.hypermesh_last_error.restype = ctypes.c_char_p

    return lib


def _encode(s: Optional[str]) -> Optional[bytes]:
    """Encode a Python string to UTF-8 bytes for C, or None -> NULL."""
    if s is None:
        return None
    return s.encode("utf-8")


class HyperMeshFFI:
    """Native FFI client using libhypermesh_ffi shared library.

    Connects to a running HyperMesh daemon over a Unix domain socket
    through the C FFI layer, bypassing HTTP entirely.

    Parameters
    ----------
    socket_path:
        Path to the daemon Unix socket. ``None`` uses the library's
        built-in 3-tier fallback (``$HYPERMESH_SOCK`` /
        ``$XDG_RUNTIME_DIR`` / ``~/.hypermesh``).
    lib_path:
        Explicit path to the shared library. When ``None``, the
        4-tier search order is used (see module docstring).
    """

    def __init__(
        self,
        socket_path: Optional[str] = None,
        lib_path: Optional[str] = None,
    ) -> None:
        resolved_path = _find_library(lib_path)
        self._lib = _load_library(resolved_path)
        self._handle: Optional[ctypes.c_void_p] = None
        self._closed = False

        handle = self._lib.hypermesh_connect(_encode(socket_path))
        if not handle:
            err = self._get_raw_error(None)
            raise FFIError(
                f"hypermesh_connect failed: {err or 'unknown error'}"
            )
        self._handle = handle

    def _get_raw_error(self, handle: Any) -> Optional[str]:
        """Read the thread-local error string without raising."""
        err_ptr = self._lib.hypermesh_last_error(handle)
        if err_ptr:
            return err_ptr.decode("utf-8", errors="replace")
        return None

    def _check_error(self) -> None:
        """Check thread-local error state and raise if set."""
        msg = self._get_raw_error(self._handle)
        if msg:
            raise FFIError(msg)

    def _call_raw(self, func: Any, *args: Any) -> Optional[bytes]:
        """Call an FFI function that returns ``char*``.

        Returns the raw bytes before freeing, or raises on NULL.
        Functions return c_void_p (int) to preserve the raw pointer for
        proper freeing — c_char_p would auto-convert and leak.
        """
        result_ptr = func(self._handle, *args)
        if not result_ptr:
            self._check_error()
            raise FFIError(
                f"{func.__name__} returned NULL with no error message"
            )
        try:
            raw = ctypes.cast(result_ptr, ctypes.c_char_p).value
        finally:
            self._lib.hypermesh_free_string(result_ptr)
        return raw

    def _call_json(self, func: Any, *args: Any) -> Any:
        """Call an FFI function and parse the JSON response."""
        raw = self._call_raw(func, *args)
        if raw is None:
            return None
        text = raw.decode("utf-8", errors="replace")
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            # Some functions return plain strings (e.g. dns_resolve)
            return text

    def _require_handle(self) -> None:
        if self._closed or self._handle is None:
            raise FFIError("Client is disconnected")

    # -----------------------------------------------------------------
    # Core API
    # -----------------------------------------------------------------

    def status(self) -> dict:
        """Fetch the current node status."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_status)

    def call(self, method: str, params: str = "{}") -> Any:
        """Send an arbitrary JSON-RPC method call to the daemon."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_call,
            _encode(method),
            _encode(params),
        )

    # -----------------------------------------------------------------
    # DNS API
    # -----------------------------------------------------------------

    def dns_resolve(self, name: str) -> Any:
        """Resolve a DNS name."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_dns_resolve, _encode(name),
        )

    def dns_list(self) -> Any:
        """List all registered DNS entries."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_dns_list)

    def dns_register(self, name: str, addr: str) -> dict:
        """Register a DNS name pointing to the given address."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_dns_register,
            _encode(name),
            _encode(addr),
        )

    # -----------------------------------------------------------------
    # Blockchain API
    # -----------------------------------------------------------------

    def blockchain_height(self) -> Any:
        """Get the current blockchain height."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_blockchain_height)

    def blockchain_block(self, index: int) -> dict:
        """Get a block by index."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_blockchain_block,
            ctypes.c_uint64(index),
        )

    # -----------------------------------------------------------------
    # Network API
    # -----------------------------------------------------------------

    def peers(self) -> Any:
        """List connected peers."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_peers)

    # -----------------------------------------------------------------
    # Topology API
    # -----------------------------------------------------------------

    def topology_info(self) -> dict:
        """Get this node's topology info."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_topology_info)

    # -----------------------------------------------------------------
    # Asset API
    # -----------------------------------------------------------------

    def asset_list(self) -> Any:
        """List all stored assets."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_asset_list)

    def asset_store(self, path: str) -> dict:
        """Store a file as a HyperMesh asset."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_asset_store, _encode(path),
        )

    def asset_fetch(self, asset_id: str, output: str) -> Any:
        """Fetch an asset by ID and write it to output path."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_asset_fetch,
            _encode(asset_id),
            _encode(output),
        )

    # -----------------------------------------------------------------
    # Domain API
    # -----------------------------------------------------------------

    def domain_list(self) -> Any:
        """List registered domains."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_domain_list)

    def domain_register(self, name: str, privacy: str = "public") -> dict:
        """Register a domain with name and privacy mode."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_domain_register,
            _encode(name),
            _encode(privacy),
        )

    # -----------------------------------------------------------------
    # Dashboard API
    # -----------------------------------------------------------------

    def dashboard_list(self) -> Any:
        """List deployed dashboards."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_dashboard_list)

    def dashboard_deploy(self, path: str) -> dict:
        """Deploy a dashboard from the given path."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_dashboard_deploy, _encode(path),
        )

    # -----------------------------------------------------------------
    # Config API
    # -----------------------------------------------------------------

    def config_show(self) -> dict:
        """Show the full daemon config."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_config_show)

    def config_get(self, key: str) -> Any:
        """Get a single config value by key."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_config_get, _encode(key),
        )

    # -----------------------------------------------------------------
    # Caesar API
    # -----------------------------------------------------------------

    def caesar_wallet(self) -> dict:
        """Fetch the caller's Caesar wallet info."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_caesar_wallet)

    def caesar_balance(self) -> dict:
        """Fetch the current Caesar balance."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_caesar_balance)

    def caesar_transactions(self, limit: int = 0) -> Any:
        """Fetch recent Caesar transactions (limit=0 for default)."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_caesar_transactions,
            ctypes.c_uint32(limit),
        )

    def caesar_rewards(self) -> dict:
        """Fetch accumulated Caesar rewards."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_caesar_rewards)

    def caesar_route_packet(self, dest: str, amount: float) -> dict:
        """Route a Caesar EVP packet to a destination."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_caesar_route_packet,
            _encode(dest),
            ctypes.c_double(amount),
        )

    def caesar_governor_params(self) -> dict:
        """Fetch current Caesar Governor parameters."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_caesar_governor_params)

    # -----------------------------------------------------------------
    # TrustChain API
    # -----------------------------------------------------------------

    def trustchain_certs(self) -> Any:
        """List all TrustChain certificates."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_trustchain_certificates)

    def trustchain_issue(self, subject: str, scope: str) -> dict:
        """Issue a new certificate for a subject and scope."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_trustchain_issue,
            _encode(subject),
            _encode(scope),
        )

    def trustchain_validate(self, cert_pem: str) -> dict:
        """Validate a PEM-encoded certificate."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_trustchain_validate, _encode(cert_pem),
        )

    def trustchain_revoke(self, cert_id: str) -> dict:
        """Revoke a certificate by ID."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_trustchain_revoke, _encode(cert_id),
        )

    def trustchain_dns_zones(self) -> Any:
        """List TrustChain DNS zones."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_trustchain_dns_zones)

    # -----------------------------------------------------------------
    # NGauge API
    # -----------------------------------------------------------------

    def ngauge_capacity(self) -> dict:
        """Fetch current node capacity metrics."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_ngauge_capacity)

    def ngauge_traffic(self) -> dict:
        """Fetch current traffic statistics."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_ngauge_traffic)

    def ngauge_marketplace(self) -> dict:
        """Fetch marketplace resource pool info."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_ngauge_marketplace)

    def ngauge_node_metrics(self) -> dict:
        """Fetch detailed node-level metrics."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_ngauge_node_metrics)

    def ngauge_leases(self) -> Any:
        """Fetch active resource leases."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_ngauge_leases)

    # -----------------------------------------------------------------
    # Catalog API
    # -----------------------------------------------------------------

    def catalog_browse(self, query: Optional[str] = None, page: int = 0) -> dict:
        """Browse catalog packages. query may be None."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_catalog_browse,
            _encode(query),
            ctypes.c_uint32(page),
        )

    def catalog_search(self, query: str) -> Any:
        """Search catalog packages by query string."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_catalog_search, _encode(query),
        )

    def catalog_package_info(self, name: str) -> dict:
        """Get detailed info about a specific catalog package."""
        self._require_handle()
        return self._call_json(
            self._lib.hypermesh_catalog_package_info, _encode(name),
        )

    def catalog_registry_stats(self) -> dict:
        """Fetch catalog registry statistics."""
        self._require_handle()
        return self._call_json(self._lib.hypermesh_catalog_registry_stats)

    # -----------------------------------------------------------------
    # Lifecycle
    # -----------------------------------------------------------------

    def close(self) -> None:
        """Disconnect and free the native client handle."""
        if not self._closed and self._handle is not None:
            self._lib.hypermesh_disconnect(self._handle)
            self._handle = None
            self._closed = True

    def __enter__(self) -> HyperMeshFFI:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()

    def __del__(self) -> None:
        self.close()

    def __repr__(self) -> str:
        state = "closed" if self._closed else "connected"
        return f"HyperMeshFFI({state})"
