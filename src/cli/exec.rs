use super::args::{Cli, Input};
use super::screenshot::take_screenshot_in_bulk;
use crate::cli::hxn_helper::combine_urls_with_ports;
use crate::log::info;
use anyhow::Context;
use chromiumoxide::{
    browser::{Browser, BrowserConfig},
    handler::viewport::Viewport,
};
use colored::Colorize;
use futures::StreamExt;
use std::sync::Arc;
use std::{env, path::Path};
use tokio::{fs, task};

pub async fn run(
    Cli {
        binary_path,
        input: Input { url, file_path },
        stdin,
        outdir,
        tabs,
        width,
        height,
        timeout,
        delay,
        verbose,
        fullpage,
        screenshot_type,
        ports,
        accept_invalid_certs,
        javascript,
    }: Cli,
) -> anyhow::Result<()> {
    let browser = Path::new(&binary_path);
    if !browser.exists() {
        return Err(anyhow::Error::msg(format!(
            "Unable to locate browser binary {binary_path}"
        )));
    }

    let (browser, mut handler) = Browser::launch(
        BrowserConfig::builder()
            .no_sandbox()
            .window_size(width, height)
            .chrome_executable(browser)
            .viewport(Viewport {
                width,
                height,
                device_scale_factor: None,
                emulating_mobile: false,
                is_landscape: false,
                has_touch: false,
            })
            .build()
            .map_err(anyhow::Error::msg)?,
    )
    .await
    .context(format!("Error instantiating browser {binary_path}"))?;
    let browser = Arc::new(browser);

    task::spawn(async move {
        while let Some(h) = handler.next().await {
            if h.is_err() {
                break;
            }
        }
    });

    let dump_dir = Path::new(&outdir);

    if dump_dir.exists() {
        info(
            format!(
                "A directory already exists as {} bumping ...",
                outdir.bold()
            ),
            colored::Color::BrightRed,
        );
    } else {
        match fs::create_dir(dump_dir).await {
            Ok(_) => info(
                format!("Bump dir {}, created successfully ..", outdir.bold()),
                colored::Color::Green,
            ),
            Err(err) => {
                info(
                    format!("Failed to create directory: {}", err),
                    colored::Color::Red,
                );
                return Err(err.into());
            }
        }
    }

    let is_screenshot_taken = if stdin {
        env::set_current_dir(dump_dir)?;
        let urls = super::hxn_helper::read_urls_from_stdin(ports)?;
        take_screenshot_in_bulk(
            &browser,
            urls,
            tabs,
            timeout,
            delay,
            verbose,
            fullpage,
            screenshot_type,
            accept_invalid_certs,
            javascript,
        )
        .await
    } else {
        match (url, file_path) {
            (None, Some(file_path)) => {
                let urls = super::hxn_helper::read_urls_from_file(file_path, ports)?;
                env::set_current_dir(dump_dir)?;
                take_screenshot_in_bulk(
                    &browser,
                    urls,
                    tabs,
                    timeout,
                    delay,
                    verbose,
                    fullpage,
                    screenshot_type,
                    accept_invalid_certs,
                    javascript,
                )
                .await
            }
            (Some(url), None) => {
                let urls = if let Some(ports) = ports {
                    combine_urls_with_ports(vec![url.to_string()], Some(ports))
                } else {
                    vec![url.to_string()]
                };
                env::set_current_dir(dump_dir)?;
                take_screenshot_in_bulk(
                    &browser,
                    urls,
                    tabs,
                    timeout,
                    delay,
                    verbose,
                    fullpage,
                    screenshot_type,
                    accept_invalid_certs,
                    javascript,
                )
                .await
            }
            _ => unreachable!(),
        }
    };

    match is_screenshot_taken {
        Ok(_) => {
            info(
                format!("Screenshots Taken and saved in directory {}", outdir.bold()),
                colored::Color::Cyan,
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}
