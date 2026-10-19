use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command},
    time::Duration,
};

#[derive(Deserialize)]
struct Lock {
    version: String,
    revision: String,
    repository: String,
    source_mirror: String,
    depot_tools: String,
    depot_revision: String,
}
pub struct Engine {
    root: PathBuf,
    checkout: PathBuf,
    tools: PathBuf,
    lock: Lock,
    output_name: String,
}
pub fn run(command: &mut Command) -> Result<()> {
    let program = command.get_program().to_string_lossy().into_owned();
    let status = command
        .status()
        .with_context(|| format!("Não foi possível executar {program}"))?;
    if !status.success() {
        bail!("Comando falhou: {program} ({status}); consulte a saída acima");
    }
    Ok(())
}
impl Engine {
    pub fn new(root: PathBuf) -> Result<Self> {
        let lock = serde_json::from_slice(&fs::read(root.join("chromium/engine.lock.json"))?)?;
        let checkout = std::env::var_os("NAVZIP_CHROMIUM_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join(".cache/chromium"));
        let checkout = if checkout.is_absolute() {
            checkout
        } else {
            root.join(checkout)
        };
        let tools = root.join(".cache/depot_tools");
        let output_name = match std::env::var("NAVZIP_CHROMIUM_OUT") {
            Ok(name) => name,
            Err(std::env::VarError::NotPresent) => "NavZip".into(),
            Err(error) => return Err(error.into()),
        };
        validate_output_name(&output_name)?;
        Ok(Self {
            root,
            checkout,
            tools,
            lock,
            output_name,
        })
    }
    fn source(&self) -> PathBuf {
        self.checkout.join("src")
    }
    fn out(&self) -> PathBuf {
        self.source().join("out").join(&self.output_name)
    }
    fn command(&self, program: &str) -> Result<Command> {
        let exe = if cfg!(windows) && self.tools.join(format!("{program}.bat")).exists() {
            self.tools.join(format!("{program}.bat"))
        } else if self.tools.join(program).exists() {
            self.tools.join(program)
        } else {
            PathBuf::from(program)
        };
        let mut c = Command::new(exe);
        c.current_dir(&self.checkout);
        let mut paths = vec![self.tools.clone()];
        if let Some(p) = std::env::var_os("NAVZIP_PYTHON") {
            paths.push(
                PathBuf::from(p)
                    .parent()
                    .context("NAVZIP_PYTHON inválido")?
                    .to_owned(),
            );
        }
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        c.env("PATH", std::env::join_paths(paths)?)
            .env("DEPOT_TOOLS_UPDATE", "0")
            .env("DEPOT_TOOLS_METRICS", "0");
        // Transport mirrors only; the exact Git revision is still verified.
        // Append process-scoped config, preserving the user's existing values.
        let inherited = std::env::var("GIT_CONFIG_COUNT")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let settings = [
            (
                format!("url.{}.insteadOf", self.lock.source_mirror),
                self.lock.repository.clone(),
            ),
            (
                "url.https://github.com/.insteadOf".into(),
                "https://chromium.googlesource.com/external/github.com/".into(),
            ),
            ("http.version".into(), "HTTP/1.1".into()),
            ("http.lowSpeedLimit".into(), "1000".into()),
            ("http.lowSpeedTime".into(), "180".into()),
        ];
        c.env("GIT_CONFIG_COUNT", (inherited + settings.len()).to_string());
        for (offset, (key, value)) in settings.into_iter().enumerate() {
            c.env(format!("GIT_CONFIG_KEY_{}", inherited + offset), key)
                .env(format!("GIT_CONFIG_VALUE_{}", inherited + offset), value);
        }
        #[cfg(target_os = "macos")]
        {
            let developer = std::env::var_os("DEVELOPER_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/Applications/Xcode.app/Contents/Developer"));
            let sdk = Command::new("xcrun")
                .env_remove("SDKROOT")
                .env("DEVELOPER_DIR", &developer)
                .args(["--sdk", "macosx", "--show-sdk-path"])
                .output()?;
            if !sdk.status.success() {
                bail!(
                    "SDK macOS indisponível: {}",
                    String::from_utf8_lossy(&sdk.stderr)
                );
            }
            c.env("DEVELOPER_DIR", developer)
                .env("SDKROOT", String::from_utf8(sdk.stdout)?.trim());
        }
        Ok(c)
    }
    pub fn doctor(&self) -> Result<()> {
        println!(
            "NavZip · Chromium {}\nRevisão: {}\nCheckout: {}",
            self.lock.version,
            self.lock.revision,
            self.checkout.display()
        );
        let available = fs2::available_space(&self.root)? / 1024 / 1024 / 1024;
        println!(
            "Disco livre: {available} GiB; sync inicial exige 120 GiB, reserva operacional 20 GiB."
        );
        if self.source().join(".git").exists() {
            self.verify_revision()?;
            println!("Commit do checkout conferido; alterações locais são mantidas.");
        }
        #[cfg(target_os = "macos")]
        {
            let mut c = self.command("xcrun")?;
            run(c.args(["--sdk", "macosx", "--show-sdk-version"]))?;
            run(self.command("xcrun")?.args(["metal", "--version"]))
                .context("Metal ausente: instale o componente MetalToolchain do Xcode")?;
        }
        Ok(())
    }
    pub fn sync(&self) -> Result<()> {
        fs::create_dir_all(&self.checkout)?;
        if !self.source().join("DEPS").exists()
            && fs2::available_space(&self.checkout)? < 120 * 1024 * 1024 * 1024
        {
            bail!("Checkout inicial requer 120 GiB livres; nenhum dado foi apagado");
        }
        if !self.tools.join(".git").exists() {
            run(Command::new("git")
                .args(["clone", "--depth", "1", &self.lock.depot_tools])
                .arg(&self.tools))?;
        }
        let current = Command::new("git")
            .arg("-C")
            .arg(&self.tools)
            .args(["rev-parse", "HEAD"])
            .output()?;
        if String::from_utf8(current.stdout)?.trim() != self.lock.depot_revision {
            run(Command::new("git").arg("-C").arg(&self.tools).args([
                "fetch",
                "--depth",
                "1",
                "origin",
                &self.lock.depot_revision,
            ]))?;
            run(Command::new("git").arg("-C").arg(&self.tools).args([
                "checkout",
                "--detach",
                &self.lock.depot_revision,
            ]))?;
        }
        self.ensure_build_tools()?;
        let config = format!(
            "solutions = [{{'name':'src','url':{:?},'managed':False,'custom_deps':{{}},'custom_vars':{{'checkout_pgo_profiles':False,'checkout_android':False,'checkout_ios':False,'checkout_fuchsia':False}}}}]\ntarget_os = []\n",
            self.lock.repository
        );
        let config_path = self.checkout.join(".gclient");
        if !config_path.exists() {
            fs::write(&config_path, config)?;
        }
        run(self.command("gclient")?.args([
            "sync",
            "--nohooks",
            "--no-history",
            "--shallow",
            "--jobs",
            "6",
            "--revision",
            &format!("src@{}", self.lock.revision),
        ]))?;
        self.verify_revision()?;
        run(self.command("gclient")?.arg("runhooks"))?;
        Ok(())
    }
    fn verify_revision(&self) -> Result<()> {
        let r = Command::new("git")
            .arg("-C")
            .arg(self.source())
            .args(["rev-parse", "HEAD"])
            .output()
            .context("Execute cargo xtask fetch primeiro")?;
        if !r.status.success() || String::from_utf8(r.stdout)?.trim() != self.lock.revision {
            bail!("Checkout Chromium não está na revisão fixada; execute cargo xtask fetch");
        }
        Ok(())
    }
    fn ensure_build_tools(&self) -> Result<()> {
        #[cfg(not(windows))]
        if !self.tools.join("python3_bin_reldir.txt").is_file() {
            // Use the absolute executable path: bootstrap changes directory,
            // so a relative depot_tools path breaks its pinned CIPD manifest.
            run(&mut self.command("ensure_bootstrap")?)?;
            if !self.tools.join("python3_bin_reldir.txt").is_file() {
                bail!("Bootstrap do Python depot_tools incompleto; consulte a saída CIPD");
            }
        }
        Ok(())
    }
    pub fn apply(&self) -> Result<()> {
        self.verify_revision()?;
        let mut patches = fs::read_dir(self.root.join("chromium/patches"))?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        patches.sort();
        for patch in patches
            .into_iter()
            .filter(|p| p.extension().is_some_and(|x| x == "patch"))
        {
            if Command::new("git")
                .arg("-C")
                .arg(self.source())
                .args(["apply", "--reverse", "--check"])
                .arg(&patch)
                .output()?
                .status
                .success()
            {
                continue;
            }
            run(Command::new("git")
                .arg("-C")
                .arg(self.source())
                .args(["apply", "--check"])
                .arg(&patch))?;
            run(Command::new("git")
                .arg("-C")
                .arg(self.source())
                .arg("apply")
                .arg(patch))?;
        }
        copy_tree(&self.root.join("chromium/overlay"), &self.source())?;
        copy_tree(
            &self.root.join("webui/newtab"),
            &self
                .source()
                .join("chrome/browser/resources/new_tab_page/navzip"),
        )?;
        let html = fs::read_to_string(self.root.join("webui/newtab/navzip.html"))?
            .replace("href=\"navzip.css\"", "href=\"navzip/navzip.css\"")
            .replace("src=\"icons/", "src=\"navzip/icons/")
            .replace(
                "src=\"navzip_newtab.js\"",
                "src=\"navzip/navzip_newtab.js\"",
            );
        write_if_changed(
            &self
                .source()
                .join("chrome/browser/resources/new_tab_page/new_tab_page.html"),
            html.as_bytes(),
        )?;
        println!(
            "Patches e recursos NavZip aplicados em {}",
            self.source().display()
        );
        Ok(())
    }
    pub fn overlay_check(&self, source: Option<PathBuf>) -> Result<()> {
        let source = source.unwrap_or_else(|| self.source());
        let mut patches = fs::read_dir(self.root.join("chromium/patches"))?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        patches.sort();
        let mut count = 0;
        for patch in patches
            .into_iter()
            .filter(|p| p.extension().is_some_and(|x| x == "patch"))
        {
            run(Command::new("git")
                .arg("-C")
                .arg(&source)
                .args(["apply", "--check"])
                .arg(patch))?;
            count += 1;
        }
        if count == 0 {
            bail!("Nenhum patch Chromium disponível");
        }
        println!("{count} patches: contexto verificado");
        Ok(())
    }
    pub fn build(&self, release: bool) -> Result<()> {
        self.build_targets(release, false)
    }
    pub fn build_integration(&self) -> Result<()> {
        self.build_targets(false, true)?;
        println!(
            "Objetos de integração e recursos compilados. O navegador não foi ligado e os testes C++ ainda não foram executados."
        );
        Ok(())
    }
    fn build_targets(&self, release: bool, integration: bool) -> Result<()> {
        self.apply()?;
        self.ensure_build_tools()?;
        if fs2::available_space(&self.checkout)? < 20 * 1024 * 1024 * 1024 {
            bail!("Reserva de 20 GiB insuficiente; nenhum dado foi apagado");
        }
        fs::create_dir_all(self.out())?;
        let arguments_changed = write_if_changed(
            &self.out().join("args.gn"),
            &fs::read(self.root.join(if release {
                "chromium/config/release.gn"
            } else {
                "chromium/config/development.gn"
            }))?,
        )?;
        if arguments_changed || !self.out().join("build.ninja").is_file() {
            run(self
                .command("gn")?
                .current_dir(self.source())
                .arg("gen")
                .arg(self.out())
                .arg("--fail-on-unused-args"))?;
        }
        // Otherwise autoninja performs GN's normal dependency checks and
        // regenerates the manifest when BUILD.gn or other GN inputs change.
        let jobs = match std::env::var("NAVZIP_BUILD_JOBS") {
            Ok(value) => {
                let jobs: usize = value
                    .parse()
                    .context("NAVZIP_BUILD_JOBS deve ser um inteiro")?;
                if !(1..=128).contains(&jobs) {
                    bail!("NAVZIP_BUILD_JOBS deve estar entre 1 e 128");
                }
                jobs
            }
            Err(std::env::VarError::NotPresent) => 6,
            Err(error) => return Err(error.into()),
        };
        let mut build = self.command("autoninja")?;
        build
            .current_dir(self.source())
            .arg("-C")
            .arg(self.out())
            .args(["-j", &jobs.to_string()]);
        if integration {
            let extension = if cfg!(windows) { "obj" } else { "o" };
            for target in [
                "chrome/browser/ui/ui/horizontal_tab_strip_region_view",
                "chrome/browser/ui/ui/vertical_tab_strip_region_view",
                "chrome/browser/ui/ui/contents_container_view",
                "chrome/browser/ui/ui/browser_commands",
                "chrome/browser/ui/bookmarks/impl/bookmark_bar_controller",
                "chrome/browser/ui/color/mixers/chrome_color_mixer",
                "chrome/browser/ui/webui/new_tab_page/impl/new_tab_page_handler",
                "chrome/browser/ui/webui/new_tab_page/impl/new_tab_page_ui",
                "chrome/browser/ui/webui/new_tab_page/unit_tests/new_tab_page_handler_unittest",
                "chrome/browser/ui/views/frame/multi_contents_view_mini_toolbar_impl/multi_contents_view_mini_toolbar",
                "chrome/browser/ui/views/frame/contents_container_outline/contents_container_outline",
                "chrome/browser/ui/views/tabs/common/impl/vertical_tab_style_views",
                "chrome/browser/ui/views/tabs/vertical/impl/vertical_tab_strip_bottom_container",
                "chrome/browser/ui/views/tabs/tab/impl/tab_title",
                "chrome/browser/ui/layout_constants/layout_constants",
            ] {
                build.arg(format!("obj/{target}.{extension}"));
            }
            build.arg("chrome/browser/resources/new_tab_page:resources");
        } else {
            build.arg("chrome");
        }
        // Keep the whole build tree stoppable when its volume fills up. Never
        // delete checkout files or unrelated user data to make room.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            build.process_group(0);
        }
        let mut child = build.spawn().context("Não foi possível iniciar o build")?;
        loop {
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    bail!("Compilação Chromium falhou ({status}); veja o erro acima");
                }
                break;
            }
            match fs2::available_space(&self.checkout) {
                Ok(bytes) if bytes >= 20 * 1024 * 1024 * 1024 => {}
                available => {
                    stop_build(&mut child);
                    bail!(
                        "Build interrompido para preservar a reserva de disco (20 GiB): {available:?}. Nenhum arquivo foi apagado; o build pode ser retomado."
                    );
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        Ok(())
    }
    fn binary(&self) -> PathBuf {
        if cfg!(target_os = "macos") {
            self.out().join("NavZip.app/Contents/MacOS/NavZip")
        } else if cfg!(windows) {
            self.out().join("chrome.exe")
        } else {
            self.out().join("chrome")
        }
    }
    pub fn capture_window(&self, pid: u32, output: &Path) -> Result<()> {
        crate::native_capture::capture(pid, &self.binary(), output)
    }
    pub fn monitor(&self) -> Result<()> {
        crate::monitor::serve(&self.root)
    }
    pub fn wait_native(&self, pid: u32) -> Result<()> {
        self.verify_revision()?;
        crate::native_pipeline::wait_and_test(pid, &self.root, &self.binary())
    }
    pub fn launch(&self, args: &[String]) -> Result<()> {
        let exe = self.binary();
        if !exe.is_file() {
            bail!(
                "Chromium/NavZip ainda não foi compilado: {}. Execute cargo xtask build. Não será aberto o Servo ou um navegador instalado.",
                exe.display()
            );
        }
        run(Command::new(exe).args(args.iter().skip_while(|a| a.as_str() == "--")))
    }
    pub fn check(&self) -> Result<()> {
        self.verify_revision()?;
        run(self
            .command("gn")?
            .current_dir(self.source())
            .arg("check")
            .arg(self.out()))
    }
    pub fn package(&self) -> Result<()> {
        if !self.binary().is_file() {
            bail!("Não existe build Chromium/NavZip; nenhum instalador foi gerado");
        }
        if !fs::read_to_string(self.out().join("args.gn"))?
            .lines()
            .any(|line| line.trim() == "is_component_build = false")
        {
            bail!(
                "O build com componentes depende de bibliotecas externas ao bundle. Execute cargo xtask build --release antes de empacotar"
            );
        }
        let stage = self.root.join("dist/chromium");
        fs::create_dir_all(&stage)?;
        #[cfg(target_os = "macos")]
        {
            if stage.join("NavZip.app").exists() {
                bail!(
                    "Saída existente: preserve/remova explicitamente dist/chromium/NavZip.app antes de empacotar"
                );
            }
            let identity = std::env::var("NAVZIP_CODESIGN_IDENTITY").unwrap_or_else(|_| "-".into());
            if identity != "-" {
                bail!(
                    "Assinatura pública exige fluxo por componente e notarização; produção ainda não configurada"
                );
            }
            run(Command::new("ditto")
                .arg(self.out().join("NavZip.app"))
                .arg(stage.join("NavZip.app")))?;
            let licenses = stage.join("NavZip.app/Contents/Resources/navzip-licenses");
            fs::create_dir_all(&licenses)?;
            for name in ["LICENSE", "THIRD_PARTY_NOTICES.md"] {
                fs::copy(self.root.join(name), licenses.join(name))?;
            }
            fs::copy(
                self.root.join("webui/newtab/icons/LICENSE-lobe-icons"),
                licenses.join("LICENSE-lobe-icons"),
            )?;
            run(Command::new("codesign")
                .args(["--force", "--deep", "--sign", "-"])
                .arg(stage.join("NavZip.app")))?;
            run(Command::new("hdiutil")
                .args([
                    "create",
                    "-fs",
                    "HFS+",
                    "-volname",
                    "NavZip experimental",
                    "-srcfolder",
                ])
                .arg(&stage)
                .arg(self.root.join("dist").join(format!(
                    "NavZip-{}-{}.dmg",
                    self.lock.version,
                    std::env::consts::ARCH
                ))))?;
            println!("Build experimental ad-hoc, não notarizado");
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            bail!("Instaladores Windows/Linux ainda não homologados; use a saída de build nativa");
        }
    }
}
fn validate_output_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        bail!("NAVZIP_CHROMIUM_OUT must be a simple directory name inside Chromium's out/");
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if entry.file_type()?.is_file() {
            write_if_changed(&target, &fs::read(entry.path())?)?;
        } else {
            bail!("Link simbólico não permitido no overlay");
        }
    }
    Ok(())
}
fn write_if_changed(path: &Path, contents: &[u8]) -> Result<bool> {
    if fs::read(path).is_ok_and(|previous| previous == contents) {
        return Ok(false);
    }
    fs::write(path, contents)?;
    Ok(true)
}

fn stop_build(child: &mut Child) {
    #[cfg(unix)]
    // SAFETY: this child was started in a fresh process group with its own PID.
    // A live child holds the PID, so it cannot be reused for another process.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGTERM);
    }
    #[cfg(windows)]
    let _ = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .status();
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod output_tests {
    use super::*;
    #[test]
    fn output_names_stay_inside_the_checkout_and_support_separate_builds() {
        for name in ["NavZip", "NavZipRelease", "release-arm64"] {
            assert!(validate_output_name(name).is_ok());
        }
        for name in [
            "",
            "../NavZip",
            "/tmp/elsewhere",
            "C:\\elsewhere",
            "out/name",
            ".",
            "..",
        ] {
            assert!(validate_output_name(name).is_err());
        }
    }
}
