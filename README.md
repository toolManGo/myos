Starting to implement my first OS based on the rCore tutorial.

## 1. 环境依赖
- RUST nightly
- QEMU 7.0以上版本
- Make
```text
qemu-system-riscv64 --version
QEMU emulator version 7.1.0
Copyright (c) 2003-2022 Fabrice Bellard and the QEMU Project developers
```

## 2. 编译运行
```shell
cd os
make run
```
## 3. 运行效果
<img src="https://github.com/toolManGo/myos/blob/master/myosshow.gif" width="428" height="240"/>