// Copyright 2026 Claude Code on behalf of Michael Yuan.
// SPDX-License-Identifier: Apache-2.0

//! # Qwen3 TTS - Rust Port
//!
//! A Rust implementation of the Qwen3 Text-to-Speech (TTS) model.
//!
//! This crate provides a high-level API for generating speech from text using
//! the Qwen3 TTS model family, which includes:
//!
//! - **CustomVoice**: Use predefined speaker voices
//! - **VoiceDesign**: Control voice characteristics with natural language
//! - **Base (VoiceClone)**: Clone voices from reference audio
//!
//! Note: the `Qwen3TTSModel` convenience wrapper remains experimental in this
//! fork and currently returns `Unsupported` for generation entrypoints. The
//! production inference path used by AlignOS is `inference::TTSInference`.
//!
//! ## Quick Start
//!
//! ```ignore
//! use qwen3_tts::inference::TTSInference;
//! use qwen3_tts::tensor::Device;
//!
//! let inference = TTSInference::new(
//!     std::path::Path::new("Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice"),
//!     Device::Cpu,
//! )?;
//!
//! let (waveform, sample_rate) = inference.generate(
//!     "Hello, welcome to Qwen TTS!",
//!     "Vivian",
//!     "english",
//!     None,
//! )?;
//!
//! // Save the output
//! qwen3_tts::audio::write_wav_file("output.wav", &waveform, sample_rate)?;
//! ```
//!
//! ## Experimental Wrapper
//!
//! ```ignore
//! use qwen3_tts::Qwen3TTSModel;
//! let model = Qwen3TTSModel::from_pretrained("Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign")?;
//!
//! let error = model
//!     .generate_voice_design("Welcome", "Warm and friendly", "english", None)
//!     .unwrap_err();
//! assert!(matches!(error, qwen3_tts::Qwen3TTSError::Unsupported(_)));
//! ```
//!
//! ## Features
//!
//! - `async`: Enable async support with tokio and reqwest for URL fetching

#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

// Ensure exactly one backend is selected
#[cfg(all(feature = "tch-backend", feature = "mlx"))]
compile_error!("Features 'tch-backend' and 'mlx' are mutually exclusive");

#[cfg(not(any(feature = "tch-backend", feature = "mlx")))]
compile_error!("Either 'tch-backend' or 'mlx' feature must be enabled");

#[cfg(feature = "mlx")]
pub mod backend;
pub mod tensor;

pub mod audio;
pub mod audio_encoder;
pub mod config;
pub mod error;
pub mod inference;
pub mod layers;
pub mod model;
pub mod speaker_encoder;
pub mod tokenizer;
pub mod types;
pub mod vocoder;
pub mod weights;

// Re-export main types at crate root for convenience
pub use audio::AudioInput;
pub use config::{GenerationConfig, Qwen3TTSConfig, TTSModelType, TokenizerType};
pub use error::{Qwen3TTSError, Result};
pub use model::{GenerationParams, Qwen3TTSModel};
pub use tokenizer::Qwen3TTSTokenizer;
pub use types::{
    DecodedAudio, EncodedAudio, GenerationOutput, Language, Speaker, VoiceClonePromptItem,
    VoiceInstruction,
};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default output sample rate in Hz.
pub const DEFAULT_SAMPLE_RATE: u32 = 24000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_default_sample_rate() {
        assert_eq!(DEFAULT_SAMPLE_RATE, 24000);
    }

    #[test]
    fn test_language_conversion() {
        // Test that "english" and "EN" both convert to Language::English
        let lang: Language = "english".into();
        assert_eq!(lang.as_str(), "english");

        let lang: Language = "EN".into();
        assert_eq!(lang.as_str(), "english");

        let lang: Language = "Auto".into();
        assert_eq!(lang.as_str(), "auto");

        let lang: Language = "zh".into();
        assert_eq!(lang.as_str(), "chinese");

        let lang = Language::Auto;
        assert_eq!(lang.as_str(), "auto");
    }

    #[test]
    fn test_speaker_creation() {
        let speaker = Speaker::new("Vivian");
        assert_eq!(speaker.name(), "Vivian");

        let speaker: Speaker = "John".into();
        assert_eq!(speaker.name(), "John");
    }

    #[test]
    fn test_audio_input_from_string() {
        let input = AudioInput::from("test.wav");
        match input {
            AudioInput::FilePath(path) => assert_eq!(path, "test.wav"),
            _ => panic!("Expected FilePath"),
        }
    }
}
