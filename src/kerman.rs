use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs::read_to_string,
    io::Error,
    path::{Path, PathBuf},
    process::Command,
};

pub static MOD_D: &str = "/lib/modules";
pub static MOD_DEP_F: &str = "modules.dep";
pub static MOD_INFO_EXE: &str = "/usr/sbin/modinfo";

/// Metadata about the kernel and details about it
#[derive(Debug, Clone)]
pub struct KernelInfo {
    pub version: String,
    path: PathBuf,
    dep_path: PathBuf,
    is_valid: bool,
    _ext: String,

    // Dependencies list in a format:
    //     "modulename" -> ["kernel/path/to/a/module.ko.zst", "kernel/other.ko.zst"]
    deplist: HashMap<String, Vec<String>>,

    // Dependencies list in a format:
    //     "modulename" -> ["module", "other"]
    lookup_deplist: HashSet<String>,
}

impl KernelInfo {
    /// Creates an instance of a KernelInfo struct with the version
    /// of the kernel and paths to required points for module analysis.
    ///
    /// Root path is either "/" for the host filesystem or a mountpoint
    /// to the root filesystem.
    ///
    /// NOTE: The module resolver is very simple here and won't scale that much
    ///       if a kernel will have millions of modules. But as of 2024 it
    ///       works OK with those dozen of thousands as for a generator.
    ///       Generated CPIO anyway will contain already sorted list.
    pub fn new(rootpath: &str, kver: &str) -> Result<Self, Error> {
        Ok(KernelInfo {
            version: kver.to_owned(),
            path: PathBuf::from(if ["", "/"].contains(&rootpath) {
                MOD_D.to_string()
            } else {
                format!("{}/{}", rootpath, MOD_D)
            }),
            dep_path: PathBuf::from(""),
            deplist: HashMap::default(),
            lookup_deplist: HashSet::default(),
            _ext: "".to_string(),
            is_valid: false,
        }
        .init()?)
    }

    /// Initialise the KernelInfo. This can be ran only once per an instance.
    fn init(mut self) -> Result<Self, Error> {
        if !self._ext.is_empty() {
            return Ok(self);
        }

        self.path = self.path.join(&self.version);
        self.dep_path = self.dep_path.join(self.path.as_os_str()).join(MOD_DEP_F);
        self.load_deps()?;

        Ok(self)
    }

    /// Return current kernel info root path.
    pub fn get_kernel_path(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }

    /// Get modules extension system: .ko[.compression]
    /// NOTE: assumption is that _all modules_ are with the same extension!
    fn get_fext(&self, fname: Option<&OsStr>) -> String {
        format!(
            ".ko{}",
            fname
                .unwrap()
                .to_owned()
                .to_str()
                .unwrap()
                .rsplit_once(".ko")
                .map_or("", |(_, l)| l)
        )
    }

    /// Load module dependencies
    /// Skip if there is no /lib/modules/<version>/kernel directory
    fn load_deps(&mut self) -> Result<(), Error> {
        if !self._ext.is_empty() {
            return Ok(());
        }

        let modpath = self.get_kernel_path().join("kernel");
        self.is_valid = Path::new(modpath.to_str().unwrap()).is_dir();
        if self.is_valid {
            for line in read_to_string(self.dep_path.as_os_str())?.lines() {
                if let Some(sl) = line.split_once(':') {
                    let (modpath, moddeps) = (sl.0.trim(), sl.1.trim());
                    if self._ext.is_empty() {
                        self._ext = self.get_fext(PathBuf::from(modpath).file_name());
                    }

                    let mut deplist: Vec<String> = vec![];
                    let mut deplist_idx: Vec<String> = vec![];

                    if !moddeps.is_empty() {
                        deplist = moddeps.split(' ').map(|x| x.to_owned()).collect();
                        deplist_idx = deplist
                            .iter()
                            .map(|x| {
                                x.split('/')
                                    .last()
                                    .unwrap()
                                    .split_once('.')
                                    .unwrap()
                                    .0
                                    .to_string()
                            })
                            .collect();
                    }

                    self.deplist.insert(modpath.to_owned(), deplist);
                    self.lookup_deplist.extend(deplist_idx.into_iter());
                }
            }
        }

        Ok(())
    }

    /// Returns true if there are actual modules on the media for this kernel.
    /// There are often kernel paths left after a kernel was not completely purged.
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Get path of dependencies file
    #[allow(dead_code)]
    pub fn get_dep_path(&self) -> &str {
        self.dep_path.to_str().unwrap()
    }

    /// Find a full path to a module
    /// Example: "sunrpc.ko" will be resolved as "kernel/net/sunrpc/sunrpc.ko"
    ///
    /// Some modules are named differently on the disk than in the memory.
    /// In this case they are tried to be resolved via external "modinfo".
    fn expand_module_name<'a>(&'a self, name: &'a String) -> &String {
        let mut m_name: String;
        if !name.ends_with(self._ext.as_str()) {
            m_name = format!("{}{}", name, self._ext); // "sunrpc" -> "sunrpc.ko"
        } else {
            m_name = name.to_owned();
        }

        if !m_name.starts_with("kernel/") {
            // name or partial path
            if !m_name.contains('/') {
                m_name = format!("/{}", m_name); // "sunrpc.ko" -> "/sunrpc.ko"
            }

            for fmodname in self.deplist.keys() {
                // Eliminate to a minimum 3rd fallback via modinfo by trying replacing underscore with a minus.
                // This not always works, because some modules called mixed.
                let mm_name = m_name.replace('_', "-");
                if fmodname.ends_with(&m_name) || fmodname.ends_with(&mm_name) {
                    return fmodname;
                }
            }
        }

        let out = Command::new(MOD_INFO_EXE).arg(name).output();
        match out {
            Ok(_) => match String::from_utf8(out.unwrap().stdout) {
                Ok(data) => {
                    for line in data.lines().map(|el| el.replace(' ', "")) {
                        if line.starts_with("filename:/") && line.contains("/kernel/") {
                            let t_modname = format!(
                                "kernel/{}",
                                line.split("/kernel/").collect::<Vec<&str>>()[1]
                            );
                            for fmodname in self.deplist.keys() {
                                if *fmodname == t_modname {
                                    return fmodname;
                                }
                            }
                        }
                    }
                }
                Err(_) => todo!(),
            },
            Err(_) => todo!(),
        }

        name
    }

    /// Resolve dependencies for one module
    /// This is an internal method
    fn get_mod_dep(&self, name: &String, mods: &mut HashSet<String>) {
        let mdeps = self.deplist.get(name).unwrap();
        for mdep in mdeps {
            mods.insert(mdep.to_owned());

            // If a dependency has its own dependencies
            let d_mdeps = self.deplist.get(mdep).unwrap();
            if !d_mdeps.is_empty() {
                for d_dep in d_mdeps {
                    mods.insert(d_dep.to_owned());
                    self.get_mod_dep(d_dep, mods);
                }
            }
        }
    }

    /// Resolve all module dependencies
    pub fn get_deps_for(&self, names: &[String]) -> HashMap<String, Vec<String>> {
        let mut mod_tree: HashMap<String, Vec<String>> = HashMap::new();
        for kmodname in names {
            let r_kmodname = self.expand_module_name(kmodname);
            if !r_kmodname.contains('/') {
                continue;
            }

            let mut mod_deps: HashSet<String> = HashSet::default();
            let mut r_deps: Vec<String> = vec![];

            self.get_mod_dep(r_kmodname, &mut mod_deps);

            for v in mod_deps {
                r_deps.push(v);
            }
            mod_tree.insert(r_kmodname.to_owned(), r_deps);
        }

        mod_tree
    }

    /// Return true if a given module is a dependency to something else
    pub fn is_dep(&self, name: &str) -> bool {
        self.lookup_deplist.contains(name)
    }

    /// Same as `get_deps_for`, except returns flattened list
    /// for all modules with their dependencies.
    pub fn get_deps_for_flatten(&self, names: &[String]) -> Vec<String> {
        let mut buff: HashSet<String> = HashSet::default();
        for (mname, mdeps) in &self.get_deps_for(names) {
            buff.insert(mname.to_owned());
            buff.extend(mdeps.to_owned());
        }

        buff.iter().map(|x| x.to_owned()).collect()
    }

    /// Get all found modules
    pub fn get_disk_modules(&self) -> Vec<String> {
        let mut buff: HashSet<String> = HashSet::default();

        for (modname, moddeps) in &self.deplist {
            buff.insert(modname.to_owned());
            buff.extend(moddeps.to_owned());
        }

        let mut mods: Vec<String> = buff.iter().map(|x| x.to_string()).collect();
        mods.sort();

        mods
    }
}
