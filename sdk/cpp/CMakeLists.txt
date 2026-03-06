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

install(TARGETS hypermesh_sdk
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)
install(DIRECTORY include/hypermesh DESTINATION include)
