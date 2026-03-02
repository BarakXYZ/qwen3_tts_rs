#!/bin/bash
# Installer for Qwen3 TTS Rust — downloads binaries, models, and reference audio
# Usage: curl -sSf https://raw.githubusercontent.com/second-state/qwen3_tts_rs/main/install.sh | bash

set -e

REPO="second-state/qwen3_tts_rs"
INSTALL_DIR="./qwen3_tts_rs"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[info]${NC}  $*"; }
ok()    { echo -e "${GREEN}[ok]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[warn]${NC}  $*"; }
err()   { echo -e "${RED}[error]${NC} $*" >&2; }

# ---------------------------------------------------------------------------
# 1. Detect platform
# ---------------------------------------------------------------------------
detect_platform() {
    case "$(uname -s)" in
        Linux*)  OS="linux" ;;
        Darwin*) OS="darwin" ;;
        *)
            err "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)    ARCH="x86_64" ;;
        aarch64|arm64)   ARCH="aarch64" ;;
        *)
            err "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac

    # CUDA detection (Linux x86_64 only)
    USE_CUDA=""
    if [ "$OS" = "linux" ] && [ "$ARCH" = "x86_64" ]; then
        if command -v nvidia-smi &>/dev/null && nvidia-smi --query-gpu=driver_version --format=csv,noheader &>/dev/null; then
            CUDA_DRIVER=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader 2>/dev/null | head -1)
            info "NVIDIA GPU detected (driver ${CUDA_DRIVER})."
            echo "  1) CUDA 12.8 (Recommended)"
            echo "  2) CPU only"
            printf "Select variant [1]: "
            read -r variant
            variant="${variant:-1}"
            if [ "$variant" = "1" ]; then
                USE_CUDA="1"
            fi
        fi
    fi

    info "Platform: ${OS} ${ARCH}${USE_CUDA:+ (CUDA)}"
}

# ---------------------------------------------------------------------------
# 2. Resolve release asset name
# ---------------------------------------------------------------------------
resolve_asset() {
    case "${OS}-${ARCH}" in
        darwin-aarch64)  ASSET_NAME="qwen3-tts-macos-aarch64" ;;
        linux-x86_64)
            if [ -n "$USE_CUDA" ]; then
                ASSET_NAME="qwen3-tts-linux-x86_64-cuda"
            else
                ASSET_NAME="qwen3-tts-linux-x86_64"
            fi
            ;;
        linux-aarch64)   ASSET_NAME="qwen3-tts-linux-aarch64" ;;
        *)
            err "Unsupported platform: ${OS}-${ARCH}"
            exit 1
            ;;
    esac
    info "Release asset: ${ASSET_NAME}"
}

# ---------------------------------------------------------------------------
# 3. Download & extract release
# ---------------------------------------------------------------------------
download_release() {
    local zip_name="${ASSET_NAME}.zip"
    local url="https://github.com/${REPO}/releases/latest/download/${zip_name}"

    info "Downloading release..."
    mkdir -p "${INSTALL_DIR}"

    local temp_dir
    temp_dir=$(mktemp -d)

    curl -fSL -o "${temp_dir}/${zip_name}" "$url"
    info "Extracting release..."
    unzip -q "${temp_dir}/${zip_name}" -d "${temp_dir}"

    # Copy everything from the extracted asset directory into INSTALL_DIR
    cp -r "${temp_dir}/${ASSET_NAME}/"* "${INSTALL_DIR}/"
    chmod +x "${INSTALL_DIR}/tts" "${INSTALL_DIR}/voice_clone"

    rm -rf "$temp_dir"
    ok "Binaries installed to ${INSTALL_DIR}/"
}

# ---------------------------------------------------------------------------
# 4. Download CUDA libtorch (Linux x86_64 CUDA only)
# ---------------------------------------------------------------------------
download_cuda_libtorch() {
    if [ -z "$USE_CUDA" ]; then
        return
    fi

    info "Downloading CUDA libtorch (this may take a while)..."
    local url="https://download.pytorch.org/libtorch/cu128/libtorch-cxx11-abi-shared-with-deps-2.7.1%2Bcu128.zip"
    local temp_dir
    temp_dir=$(mktemp -d)

    curl -fSL -o "${temp_dir}/libtorch.zip" "$url"
    info "Extracting libtorch..."
    unzip -q "${temp_dir}/libtorch.zip" -d "${temp_dir}"

    rm -rf "${INSTALL_DIR}/libtorch"
    mv "${temp_dir}/libtorch" "${INSTALL_DIR}/libtorch"

    rm -rf "$temp_dir"
    ok "CUDA libtorch installed to ${INSTALL_DIR}/libtorch/"
}

# ---------------------------------------------------------------------------
# 5. Model selection & download
# ---------------------------------------------------------------------------
download_models() {
    echo ""
    info "Select model size:"
    echo "  1) 0.6B (Recommended)"
    echo "  2) 1.7B"
    printf "Select model [1]: "
    read -r model_choice
    model_choice="${model_choice:-1}"

    if [ "$model_choice" = "2" ]; then
        MODEL_SIZE="1.7B"
        CUSTOM_VOICE_MODEL="Qwen3-TTS-12Hz-1.7B-CustomVoice"
    else
        MODEL_SIZE="0.6B"
        CUSTOM_VOICE_MODEL="Qwen3-TTS-12Hz-0.6B-CustomVoice"
    fi
    # Base model is always 0.6B (no 1.7B Base exists)
    BASE_MODEL="Qwen3-TTS-12Hz-0.6B-Base"

    local models_dir="${INSTALL_DIR}/models"
    mkdir -p "$models_dir"

    # Ensure huggingface_hub and transformers are available
    if ! command -v huggingface-cli &>/dev/null; then
        info "Installing huggingface_hub and transformers..."
        pip install -q huggingface_hub transformers
    fi

    for model in "$CUSTOM_VOICE_MODEL" "$BASE_MODEL"; do
        local model_dir="${models_dir}/${model}"
        if [ -d "$model_dir" ] && [ -n "$(ls -A "$model_dir" 2>/dev/null)" ]; then
            ok "${model} already downloaded, skipping."
        else
            info "Downloading ${model}..."
            huggingface-cli download "Qwen/${model}" --local-dir "$model_dir"
            ok "${model} downloaded."
        fi
    done

    # Generate tokenizer.json files
    info "Generating tokenizer.json files..."
    python3 -c "
from transformers import AutoTokenizer
for model in ['${CUSTOM_VOICE_MODEL}', '${BASE_MODEL}']:
    path = '${models_dir}/' + model
    tok = AutoTokenizer.from_pretrained(path, trust_remote_code=True)
    tok.backend_tokenizer.save(path + '/tokenizer.json')
    print(f'Saved {path}/tokenizer.json')
"
    ok "Tokenizers generated."
}

# ---------------------------------------------------------------------------
# 6. Download reference audio
# ---------------------------------------------------------------------------
download_reference_audio() {
    local ref_dir="${INSTALL_DIR}/reference_audio"
    mkdir -p "$ref_dir"

    local base_url="https://raw.githubusercontent.com/${REPO}/main/reference_audio"

    info "Downloading reference audio..."
    for file in trump.wav trump.txt elon_musk.wav elon_musk.txt; do
        if [ ! -f "${ref_dir}/${file}" ]; then
            curl -fSL -o "${ref_dir}/${file}" "${base_url}/${file}"
        fi
    done
    ok "Reference audio saved to ${ref_dir}/"
}

# ---------------------------------------------------------------------------
# 7. Done — print sample commands
# ---------------------------------------------------------------------------
print_usage() {
    local cv_model_path="models/${CUSTOM_VOICE_MODEL}"
    local base_model_path="models/${BASE_MODEL}"

    echo ""
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN} Installation complete!${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""

    echo "Text-to-Speech:"
    echo ""
    echo "  cd ${INSTALL_DIR}"
    echo "  ./tts ${cv_model_path} \"Hello! This is a test of Qwen3 TTS.\" Vivian english"
    echo ""

    echo "Voice Cloning (ICL mode):"
    echo ""
    echo "  cd ${INSTALL_DIR}"
    echo "  ./voice_clone ${base_model_path} reference_audio/trump.wav \\"
    echo "    \"Hello, this is a voice cloning test.\" english \\"
    echo "    \"Angered and appalled millions of Americans across the political spectrum\""
    echo ""
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    echo ""
    info "Qwen3 TTS Rust Installer"
    echo ""

    detect_platform
    resolve_asset
    download_release
    download_cuda_libtorch
    download_models
    download_reference_audio
    print_usage
}

main "$@"
