use anyhow::{bail, Result};
use backtrace::Backtrace;
use chrono::Local;
use lettre::{transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
use log::error;
use std::{
    env::{current_exe, var},
    ffi::OsStr,
    fs,
    panic::{set_hook, Location},
    path::PathBuf,
    process::{exit, Command},
};

pub struct PanicHandler {
    mail: Option<Mail>,
    dump: String,
    exe: String,
    restart: bool,
}

struct Mail {
    sender: SmtpTransport,
    account: String,
    targets: Vec<String>,
}

impl Mail {
    fn new() -> Self {
        let server = var("PANIC_MAIL_SRV").unwrap_or("mail.dahuatech.com".to_owned());
        let account = var("PANIC_MAIL_ACCOUNT").unwrap_or("qdrj_noreply@dahuatech.com".to_owned());
        let password = var("PANIC_MAIL_PASS").unwrap_or("Selina12#".to_string());
        let sender = SmtpTransport::builder_dangerous(server)
            .credentials(Credentials::new(account.clone(), password))
            .build();

        Self {
            sender,
            account,
            targets: Vec::new(),
        }
    }

    fn send_mail(&self, subject: impl Into<String>, body: impl Into<String>) -> Result<()> {
        let subject = subject.into();
        let body = body.into();
        for m in &self.targets {
            let mail = Message::builder()
                .from(self.account.parse()?)
                .to(m.parse()?)
                .subject(subject.clone())
                .body(body.clone())?;
            let result = self.sender.send(&mail)?;
            if !result.is_positive() {
                bail!(
                    "send notify mail failed with {}: {}",
                    result.code(),
                    result.message().collect::<Vec<_>>().join("\n")
                );
            }
        }

        Ok(())
    }
}

impl PanicHandler {
    pub fn new() -> Self {
        let exe = current_exe().unwrap_or(PathBuf::from("<unknown>"));
        let exe = exe
            .file_stem()
            .unwrap_or(OsStr::new("<unknown>"))
            .to_string_lossy();
        Self {
            mail: None,
            dump: format!("Panic_{}", exe),
            exe: exe.to_string(),
            restart: false,
        }
    }

    pub fn setup(self) {
        set_hook(Box::new(move |panic_info| {
            let bt = Backtrace::new();
            let dump = format!(
                "{:?}\nthread '{}' panicked at '{}' {}",
                bt,
                std::thread::current().name().unwrap_or("<unnamed>"),
                panic_info
                    .payload()
                    .downcast_ref::<&str>()
                    .unwrap_or(&"<unknown>"),
                panic_info.location().unwrap_or(Location::caller())
            );

            let dump_file = format!(
                "{}_{}.dump",
                self.dump,
                Local::now().format("%Y%m%d_%H%M%S")
            );

            if let Err(e) = fs::write(dump_file, dump.as_bytes()) {
                error!("write panic report file error: {:?}", e);
            }

            if let Some(mail) = self.mail.as_ref() {
                if let Err(e) = mail.send_mail(format!("{} is PANIC!!", self.exe), &dump) {
                    error!("send panic report mail error: {:?}", e);
                }
            }

            if self.restart {
                let args: Vec<_> = std::env::args().collect();
                if let Err(e) = Command::new(&args[0]).args(&args[1..]).spawn() {
                    error!("restart program {} error: {:?}", self.exe, e);
                }
            }

            exit(-1);
        }));
    }

    pub fn add_mail(mut self, target: impl Into<String>) -> Self {
        if self.mail.is_none() {
            self.mail = Some(Mail::new());
        }

        self.mail.as_mut().unwrap().targets.push(target.into());
        self
    }

    pub fn custom_dump_file(mut self, p: impl Into<String>) -> Self {
        self.dump = p.into();
        self
    }

    pub fn enable_restart(mut self) -> Self {
        self.restart = true;
        self
    }
}
