use std::process::Command;

pub struct Systemctl {
    systemctl: Command,
}

impl Systemctl {
    pub fn new() -> Self {
        Self {
            systemctl: Command::new("systemctl"),
        }
    }

    pub fn enable(&mut self, service: &str) -> &mut Self {
        self.systemctl.arg("--user").arg("enable").arg(service);
        self
    }

    pub fn start(&mut self, service: &str) -> &mut Self {
        self.systemctl.arg("--user").arg("start").arg(service);
        self
    }

    pub fn stop(&mut self, service: &str) -> &mut Self {
        self.systemctl.arg("--user").arg("stop").arg(service);
        self
    }

    pub fn restart(&mut self, service: &str) -> &mut Self {
        self.systemctl.arg("--user").arg("restart").arg(service);
        self
    }

    pub fn status(&mut self, service: &str) -> &mut Self {
        self.systemctl.arg("--user").arg("status").arg(service);
        self
    }

    pub fn disable(&mut self, service: &str) -> &mut Self {
        self.systemctl.arg("--user").arg("disable").arg(service);
        self
    }

    pub fn daemon_reload(&mut self) -> &mut Self {
        self.systemctl.arg("--user").arg("daemon-reload");
        self
    }

    pub fn reset_failed(&mut self) -> &mut Self {
        self.systemctl.arg("--user").arg("reset-failed");
        self
    }

    pub fn execute(&mut self) {
        self.systemctl.spawn().expect("failed to execute process");
    }
}
