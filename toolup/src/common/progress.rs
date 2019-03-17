use indicatif::{ProgressBar, ProgressStyle};

pub struct ProgressBarHelper {
    pb: Option<ProgressBar>,
}

pub enum ProgressBarType {
    Downloading,
    Updating,
}

impl ProgressBarHelper {
    pub fn new(len: u64, p_type: ProgressBarType) -> Self {
        if atty::isnt(atty::Stream::Stdout) {
            ProgressBarHelper { pb: None }
        } else {
            let template = match p_type {
                ProgressBarType::Downloading => {
                    "{prefix:.bold.dim} {spinner} [{pos}/{len}] Downloading {wide_msg}"
                }
                ProgressBarType::Updating => {
                    "{prefix:.bold.dim} {spinner} [{pos}/{len}] Updating {wide_msg}"
                }
            };

            let pb = ProgressBar::new(len);
            let spinner_style = ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                .template(template);
            pb.set_style(spinner_style.clone());
            pb.enable_steady_tick(100);
            ProgressBarHelper { pb: Some(pb) }
        }
    }

    pub fn inc(&self, message: &str) {
        if let Some(pb) = &self.pb {
            pb.inc(1);
            pb.set_message(message);
        }
    }

    pub fn done(&self) {
        if let Some(pb) = &self.pb {
            pb.finish_and_clear();
        }
    }
}
