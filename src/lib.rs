use kerman::{KernelInfo, MOD_D};
use std::fs::read_dir;
mod kerman;
mod moddeps;
pub mod modinfo;

/// Get the list of existing kernels in the system.
pub fn get_kernel_infos(rootfs: &str) -> Vec<KernelInfo> {
    let mut kernels: Vec<KernelInfo> = vec![];
    for fres in read_dir(MOD_D).unwrap() {
        let fd = fres.unwrap();
        if fd.file_type().unwrap().is_dir() {
            let kinfo: KernelInfo =
                KernelInfo::new(rootfs, fd.path().file_name().unwrap().to_str().unwrap());
            if kinfo.is_valid() {
                kernels.push(kinfo);
            }
        }
    }

    kernels
}
