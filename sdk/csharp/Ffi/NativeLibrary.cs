using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace HyperMesh.Sdk.Ffi;

/// <summary>
/// Cross-platform native library resolver for libhypermesh_ffi.
/// Searches: explicit path > HYPERMESH_FFI_LIB env > common build paths.
/// </summary>
internal static class NativeLibraryResolver
{
    private const string LibName = "hypermesh_ffi";

    /// <summary>
    /// Register the DllImport resolver for the hypermesh_ffi library.
    /// Must be called before any P/Invoke into the library.
    /// </summary>
    internal static void Register(string? explicitPath = null)
    {
        System.Runtime.InteropServices.NativeLibrary.SetDllImportResolver(
            typeof(NativeLibraryResolver).Assembly,
            (name, assembly, searchPath) =>
            {
                if (name != LibName)
                    return IntPtr.Zero;

                return ResolveLibrary(explicitPath);
            });
    }

    private static IntPtr ResolveLibrary(string? explicitPath)
    {
        // 1. Explicit path provided by caller
        if (!string.IsNullOrEmpty(explicitPath) && File.Exists(explicitPath))
        {
            if (System.Runtime.InteropServices.NativeLibrary.TryLoad(explicitPath, out var handle))
                return handle;
        }

        // 2. HYPERMESH_FFI_LIB environment variable
        var envPath = Environment.GetEnvironmentVariable("HYPERMESH_FFI_LIB");
        if (!string.IsNullOrEmpty(envPath) && File.Exists(envPath))
        {
            if (System.Runtime.InteropServices.NativeLibrary.TryLoad(envPath, out var handle))
                return handle;
        }

        // 3. Platform-specific default name (system search paths)
        var platformName = GetPlatformLibraryName();
        if (System.Runtime.InteropServices.NativeLibrary.TryLoad(platformName, out var sysHandle))
            return sysHandle;

        // 4. Common build output paths relative to the assembly location
        var assemblyDir = Path.GetDirectoryName(
            typeof(NativeLibraryResolver).Assembly.Location) ?? ".";

        foreach (var candidate in GetSearchPaths(assemblyDir, platformName))
        {
            if (File.Exists(candidate) &&
                System.Runtime.InteropServices.NativeLibrary.TryLoad(candidate, out var h))
                return h;
        }

        // 5. Fall back to default runtime resolution (will throw DllNotFoundException
        //    with a clear message if nothing works)
        return System.Runtime.InteropServices.NativeLibrary.Load(platformName);
    }

    private static string GetPlatformLibraryName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return "hypermesh_ffi.dll";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return "libhypermesh_ffi.dylib";
        return "libhypermesh_ffi.so";
    }

    private static IEnumerable<string> GetSearchPaths(string assemblyDir, string libFile)
    {
        // Adjacent to the managed assembly
        yield return Path.Combine(assemblyDir, libFile);

        // runtimes/<rid>/native/ (NuGet native layout)
        var rid = RuntimeInformation.RuntimeIdentifier;
        yield return Path.Combine(assemblyDir, "runtimes", rid, "native", libFile);

        // Cargo target directories (debug + release)
        var coreRoot = FindCoreRoot(assemblyDir);
        if (coreRoot != null)
        {
            yield return Path.Combine(coreRoot, "target", "release", libFile);
            yield return Path.Combine(coreRoot, "target", "debug", libFile);
        }

        // /usr/local/lib
        yield return Path.Combine("/usr", "local", "lib", libFile);
    }

    /// <summary>
    /// Walk up from assemblyDir looking for a directory that contains Cargo.toml,
    /// which indicates the HyperMesh core workspace root.
    /// </summary>
    private static string? FindCoreRoot(string startDir)
    {
        var dir = startDir;
        for (var i = 0; i < 8 && dir != null; i++)
        {
            if (File.Exists(Path.Combine(dir, "Cargo.toml")))
                return dir;
            dir = Path.GetDirectoryName(dir);
        }
        return null;
    }
}
