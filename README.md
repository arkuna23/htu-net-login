# htu-net-login

河师大校园网自动登录

## 功能

- 自动登录校园网
- 持续检查校园网是否可用，断线自动重新登录

## 安装

### Windows

1. 在[Releases页面](https://github.com/arkuna23/htu-net-login/releases)下载，下载htu-net-login.exe

2. 双击下载的文件进行运行，之后会自动安装，按照提示重启并双击创建的桌面快捷方式来配置自动登录账号(设定账号选项中设定)

3. 配置完成后，每当连接到校园网并且无法连接到网络时就会尝试登录，登录成功会发送通知。

## 其他

### 如何卸载?

#### Windows下

打开终端，输入`htu-net --uninstall-daemon`指令删除自动登录的自动启动，之后删除`C:\Windows\htu-net.exe`和桌面快捷方式即可。

另外，配置文件和日志都在`C:\Users\<你的用户名>\AppData\Roaming\htu-net`目录下
