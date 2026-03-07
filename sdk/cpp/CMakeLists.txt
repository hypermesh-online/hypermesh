cmake_minimum_required(VERSION 3.14)
project(hypermesh-sdk-cpp VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

include(FetchContent)
FetchContent_Declare(
    nlohmann_json
    GIT_REPOSITORY https://github.com/nlohmann/json.git
    GIT_TAG        v3.11.3
    GIT_SHALLOW    TRUE
)
FetchContent_MakeAvailable(nlohmann_json)

set(SDK_SOURCES
    src/http_client.cpp
    src/client.cpp
    src/api/node.cpp
    src/api/blockchain.cpp
    src/api/dns.cpp
    src/api/network.cpp
    src/api/topology.cpp
    src/api/asset.cpp
    src/api/dashboard.cpp
    src/api/config.cpp
    src/api/domain.cpp
    src/api/caesar.cpp
    src/api/trustchain.cpp
    src/api/engauge.cpp
    src/api/catalog.cpp
)

add_library(hypermesh_sdk ${SDK_SOURCES})
target_include_directories(hypermesh_sdk PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
)
target_link_libraries(hypermesh_sdk PUBLIC nlohmann_json::nlohmann_json)

add_library(hypermesh::sdk ALIAS hypermesh_sdk)

# ---------------------------------------------------------------------------
# FFI client — links directly to libhypermesh_ffi (no HTTP overhead).
#
# The FFI shared library must be installed or its path added via
# CMAKE_PREFIX_PATH / HYPERMESH_FFI_DIR.  Build with:
#   cmake -DHYPERMESH_FFI_DIR=/path/to/hypermesh-ffi/target/release ..
# ---------------------------------------------------------------------------
option(HYPERMESH_BUILD_FFI "Build the FFI client (requires libhypermesh_ffi)" ON)

if(HYPERMESH_BUILD_FFI)
    add_library(hypermesh_ffi_client src/ffi_client.cpp)

    target_include_directories(hypermesh_ffi_client PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../../hypermesh-ffi/include>
        $<INSTALL_INTERFACE:include>
    )

    # Locate or accept an explicit path to libhypermesh_ffi.
    if(DEFINED HYPERMESH_FFI_DIR)
        target_link_directories(hypermesh_ffi_client PUBLIC ${HYPERMESH_FFI_DIR})
    endif()

    target_link_libraries(hypermesh_ffi_client PUBLIC hypermesh_ffi)

    add_library(hypermesh::ffi_client ALIAS hypermesh_ffi_client)

    install(TARGETS hypermesh_ffi_client
        ARCHIVE DESTINATION lib
        LIBRARY DESTINATION lib
    )
endif()

install(TARGETS hypermesh_sdk
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)
install(DIRECTORY include/hypermesh DESTINATION include)
