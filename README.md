# kmoddep

A tiny library to work with Linux kernel module information. It currently features the following:

- finding all available kernels
- find module dependencies
- lsmod (as a function)

# Usage Example

```[rust]
use kmoddep::modinfo::lsmod;

fn main() {
    for m in lsmod() {
        println!("{:<30} {:<10} {} {}", m.name, m.mem_size, m.instances, m.dependencies.join(", ");
    }
}
```
