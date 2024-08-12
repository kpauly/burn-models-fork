use std::time::Instant;

use burn::tensor::{backend::Backend, Device};
use clap::Parser;
use llama_burn::{
    llama::{Llama, LlamaConfig},
    sampling::{Sampler, TopP},
    tokenizer::Tokenizer,
};

const DEFAULT_PROMPT: &str = "How many helicopters can a human eat in one sitting?";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Top-p probability threshold.
    #[arg(long, default_value_t = 0.9)]
    top_p: f64,

    /// Temperature value for controlling randomness in sampling.
    #[arg(long, default_value_t = 0.6)]
    temperature: f64,

    /// Maximum sequence length for input text.
    #[arg(long, default_value_t = 128)]
    max_seq_len: usize,

    /// The number of new tokens to generate (i.e., the number of generation steps to take).
    #[arg(long, short = 'n', default_value_t = 50)]
    sample_len: usize,

    /// The seed to use when generating random samples.
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// The input prompt.
    #[arg(short, long, default_value_t = String::from(DEFAULT_PROMPT))]
    prompt: String,
}

pub fn generate<B: Backend, T: Tokenizer>(
    llama: &mut Llama<B, T>,
    prompt: &str,
    sample_len: usize,
    temperature: f64,
    sampler: &mut Sampler,
) {
    let now = Instant::now();
    let generated = llama.generate(prompt, sample_len, temperature, sampler);
    let elapsed = now.elapsed().as_secs();

    println!("> {}\n", generated.text);
    println!(
        "{} tokens generated ({:.4} tokens/s)\n",
        generated.tokens,
        generated.tokens as f64 / generated.time
    );

    println!(
        "Generation completed in {}m{}s",
        (elapsed / 60),
        elapsed % 60
    );
}

pub fn chat<B: Backend>(args: Config, device: Device<B>) {
    let mut prompt = args.prompt;

    // Sampling strategy
    let mut sampler = if args.temperature > 0.0 {
        Sampler::TopP(TopP::new(args.top_p, args.seed))
    } else {
        Sampler::Argmax
    };

    #[cfg(feature = "tiny")]
    {
        // TinyLlama-1.1B Chat v1.0
        let mut llama = LlamaConfig::tiny_llama_pretrained::<B>(&device).unwrap();
        println!("Processing prompt: {}", prompt);

        // Prompt formatting for chat model
        prompt = format!(
            "<|system|>\nYou are a friendly chatbot who always responds in the style of a pirate</s>\n<|user|>\n{prompt}</s>\n<|assistant|>\n"
        );

        generate(
            &mut llama,
            &prompt,
            args.sample_len,
            args.temperature,
            &mut sampler,
        );
    }

    #[cfg(feature = "llama3")]
    {
        // Llama-3-8B-Instruct
        let mut llama = LlamaConfig::llama3_8b_pretrained::<B>(true, &device).unwrap();
        println!("Processing prompt: {}", prompt);

        // Prompt formatting for chat model
        prompt = format!(
            "<|start_header_id|>system<|end_header_id|>\n\nA chat between a curious user and an artificial intelligence assistant. The assistant gives helpful, detailed, and polite answers to the user's questions.<|eot_id|><|start_header_id|>user<|end_header_id|>\n\n{prompt}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n"
        );

        generate(
            &mut llama,
            &prompt,
            args.sample_len,
            args.temperature,
            &mut sampler,
        );
    }
}

#[cfg(feature = "tch-gpu")]
mod tch_gpu {
    use super::*;
    use burn::{
        backend::{libtorch::LibTorchDevice, LibTorch},
        tensor::f16,
    };

    pub fn run(args: Config) {
        #[cfg(not(target_os = "macos"))]
        let device = LibTorchDevice::Cuda(0);
        #[cfg(target_os = "macos")]
        let device = LibTorchDevice::Mps;

        chat::<LibTorch<f16>>(args, device);
    }
}

#[cfg(feature = "tch-cpu")]
mod tch_cpu {
    use super::*;
    use burn::backend::{libtorch::LibTorchDevice, LibTorch};

    pub fn run(args: Config) {
        let device = LibTorchDevice::Cpu;

        chat::<LibTorch>(args, device);
    }
}

#[cfg(feature = "wgpu")]
mod wgpu {
    use super::*;
    use burn::backend::wgpu::{Wgpu, WgpuDevice};

    pub fn run(args: Config) {
        let device = WgpuDevice::default();

        chat::<Wgpu>(args, device);
    }
}

pub fn main() {
    // Parse arguments
    let args = Config::parse();

    #[cfg(feature = "tch-gpu")]
    tch_gpu::run(args);
    #[cfg(feature = "tch-cpu")]
    tch_cpu::run(args);
    #[cfg(feature = "wgpu")]
    wgpu::run(args);
}
