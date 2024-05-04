pub mod kerman;
pub mod moddeps;
pub mod modinfo;

use kerman::{KernelInfo, MOD_D};
use std::{fs::read_dir, io::Error};

/// Get the list of existing kernels in the system.
pub fn get_kernel_infos(rootfs: Option<&str>) -> Result<Vec<KernelInfo>, Error> {
    let mut rfs_path = "";
    if let Some(mut rootfs) = rootfs {
        rootfs = rootfs.trim().trim_end_matches("/");
        if !rootfs.is_empty() && !rootfs.eq("/") {
            rfs_path = rootfs;
        }
    }

    let mut kernels: Vec<KernelInfo> = vec![];

    for fres in read_dir(format!("{}{}", rfs_path.trim_end_matches("/"), MOD_D)).unwrap() {
        let fd = fres.unwrap();
        if fd.file_type().unwrap().is_dir() {
            let kinfo: KernelInfo =
                KernelInfo::new(rfs_path, fd.path().file_name().unwrap().to_str().unwrap())?;
            if kinfo.is_valid() {
                kernels.push(kinfo);
            }
        }
    }

    Ok(kernels)
}
