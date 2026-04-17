use anyhow::{anyhow, Context, Result};
use clap::Parser;
use libmathcat::interface::{get_spoken_text, set_mathml, set_preference, set_rules_dir};
use reqwest::blocking::Client;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(author, version, about = "LaTeX -> MathJax -> MathML -> MathCAT SSML -> Azure TTS")]
struct Args {
    /// Raw LaTeX expression (do not include surrounding $ delimiters)
    latex: String,

    /// Output audio file (.mp3)
    #[arg(long)]
    out: PathBuf,

    /// Path to the Node helper
    #[arg(long, default_value = "./tex2mml.mjs")]
    tex2mml: PathBuf,

    /// Path to the MathCAT Rules directory
    #[arg(long)]
    rules_dir: PathBuf,

    /// Azure Speech region (optional if AZURE_SPEECH_REGION is set)
    #[arg(long)]
    azure_region: Option<String>,

    /// Azure Speech key (optional if AZURE_SPEECH_KEY is set)
    #[arg(long)]
    azure_key: Option<String>,

    /// Azure voice name
    #[arg(long, default_value = "en-US-JennyNeural")]
    voice: String,

    /// Optional: write intermediate MathML to this file
    #[arg(long)]
    save_mathml: Option<PathBuf>,

    /// Optional: write final SSML to this file
    #[arg(long)]
    save_ssml: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let azure_region = args
        .azure_region
        .or_else(|| std::env::var("AZURE_SPEECH_REGION").ok())
        .context("Provide --azure-region or set AZURE_SPEECH_REGION")?;

    let azure_key = args
        .azure_key
        .or_else(|| std::env::var("AZURE_SPEECH_KEY").ok())
        .context("Provide --azure-key or set AZURE_SPEECH_KEY")?;

    let mathml = latex_to_mathml(&args.tex2mml, &args.latex)
        .context("Converting LaTeX to MathML with MathJax failed")?;

    if let Some(path) = &args.save_mathml {
        fs::write(path, &mathml)
            .with_context(|| format!("Writing MathML to {}", path.display()))?;
    }

    let ssml_fragment = mathml_to_mathcat_ssml_fragment(&args.rules_dir, &mathml)
        .context("Converting MathML to MathCAT SSML failed")?;

    let ssml = wrap_ssml_fragment(&ssml_fragment, &args.voice);

    if let Some(path) = &args.save_ssml {
        fs::write(path, &ssml)
            .with_context(|| format!("Writing SSML to {}", path.display()))?;
    }

    synthesize_with_azure(&azure_region, &azure_key, &ssml, &args.out)
        .context("Synthesizing audio from SSML failed")?;

    eprintln!("Wrote {}", args.out.display());
    Ok(())
}

fn latex_to_mathml(script: &Path, latex: &str) -> Result<String> {
    let output = Command::new("node")
        .arg(script)
        .arg(latex)
        .output()
        .with_context(|| format!("Running node {}", script.display()))?;

    if !output.status.success() {
        return Err(anyhow!(
            "MathJax conversion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let mathml = String::from_utf8(output.stdout)
        .context("MathJax output was not valid UTF-8")?;

    Ok(mathml)
}

fn mathml_to_mathcat_ssml_fragment(rules_dir: &Path, mathml: &str) -> Result<String> {
    set_rules_dir(
        rules_dir
            .to_str()
            .ok_or_else(|| anyhow!("rules_dir is not valid UTF-8"))?
            .to_string(),
    )?;

    // These names reflect the intended preferences for your use case.
    // If your local MathCAT build uses slightly different canonical names,
    // adjust them here.
    set_preference("Language".to_string(), "en".to_string())?;
    set_preference("SpeechStyle".to_string(), "ClearSpeak".to_string())?;
    set_preference("Verbosity".to_string(), "Verbose".to_string())?;
    set_preference("Impairment".to_string(), "Blindness".to_string())?;
    set_preference("TTS".to_string(), "SSML".to_string())?;

    set_mathml(mathml.to_string())?;
    let ssml_fragment = get_spoken_text()?;
    Ok(ssml_fragment)
}

fn wrap_ssml_fragment(ssml_fragment: &str, voice: &str) -> String {
    format!(
        r#"<speak version="1.0" xml:lang="en-US">
  <voice name="{voice}">
    {ssml_fragment}
  </voice>
</speak>
"#
    )
}

fn synthesize_with_azure(region: &str, key: &str, ssml: &str, out: &Path) -> Result<()> {
    let url = format!(
        "https://{}.tts.speech.microsoft.com/cognitiveservices/v1",
        region
    );

    let client = Client::new();
    let response = client
        .post(url)
        .header("Ocp-Apim-Subscription-Key", key)
        .header("Content-Type", "application/ssml+xml")
        .header("X-Microsoft-OutputFormat", "audio-24khz-160kbitrate-mono-mp3")
        .body(ssml.to_string())
        .send()?
        .error_for_status()?;

    let bytes = response.bytes()?;
    fs::write(out, &bytes)
        .with_context(|| format!("Writing audio to {}", out.display()))?;

    Ok(())
}
