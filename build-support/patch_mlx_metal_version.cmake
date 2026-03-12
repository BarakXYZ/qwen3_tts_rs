if(NOT DEFINED MLX_SOURCE_DIR)
  message(FATAL_ERROR "MLX_SOURCE_DIR must be provided to patch_mlx_metal_version.cmake")
endif()

set(DEVICE_CPP "${MLX_SOURCE_DIR}/mlx/backend/metal/device.cpp")
if(NOT EXISTS "${DEVICE_CPP}")
  message(FATAL_ERROR "MLX device.cpp not found at ${DEVICE_CPP}")
endif()

file(READ "${DEVICE_CPP}" DEVICE_CPP_CONTENTS)

set(DEVICE_CPP_OLD "if (__builtin_available(macOS 26, iOS 26, tvOS 26, visionOS 26, *)) {\n      return MTL::LanguageVersion4_0;\n    } else if (__builtin_available(macOS 15, iOS 18, tvOS 18, visionOS 2, *)) {\n      return MTL::LanguageVersion3_2;\n    } else {\n      return MTL::LanguageVersion3_1;\n    }")
set(DEVICE_CPP_NEW "if (__builtin_available(macOS 15, iOS 18, tvOS 18, visionOS 2, *)) {\n      return MTL::LanguageVersion3_2;\n    } else {\n      return MTL::LanguageVersion3_1;\n    }")

if(DEVICE_CPP_CONTENTS MATCHES "LanguageVersion4_0")
  string(REPLACE "${DEVICE_CPP_OLD}" "${DEVICE_CPP_NEW}" DEVICE_CPP_CONTENTS "${DEVICE_CPP_CONTENTS}")
  file(WRITE "${DEVICE_CPP}" "${DEVICE_CPP_CONTENTS}")
  message(STATUS "Patched MLX Metal language selection to cap source compilation at Metal 3.2")
else()
  message(STATUS "MLX Metal language selection already capped; no patch needed")
endif()
