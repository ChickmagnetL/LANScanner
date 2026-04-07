use std::path::PathBuf;

use crate::visual_check::VisualCheckConfig;

pub enum LaunchMode {
    Normal,
    VisualCheck(VisualCheckConfig),
}

pub fn parse_launch_mode_from_env() -> Result<LaunchMode, String> {
    let mut args = std::env::args().skip(1);
    let mut scene: Option<String> = None;
    let mut output_dir: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--visual-check" => {
                let Some(value) = args.next() else {
                    return Err(String::from(
                        "missing value for --visual-check. usage: --visual-check <scene|all>",
                    ));
                };
                scene = Some(value);
            }
            "--output-dir" => {
                let Some(value) = args.next() else {
                    return Err(String::from(
                        "missing value for --output-dir. usage: --output-dir <path>",
                    ));
                };
                output_dir = Some(PathBuf::from(value));
            }
            _ => {
                return Err(format!(
                    "unknown argument `{arg}`. expected: --visual-check <scene|all> [--output-dir <path>]"
                ));
            }
        }
    }

    let Some(scene) = scene else {
        return Ok(LaunchMode::Normal);
    };

    let config = VisualCheckConfig::from_args(&scene, output_dir)?;
    Ok(LaunchMode::VisualCheck(config))
}
