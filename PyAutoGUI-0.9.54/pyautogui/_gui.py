import sys
import os
import tkinter as tk
from tkinter import messagebox
import ctypes

def show_window():
    from . import set_use_driver
    
    def on_toggle():
        val = var.get()
        set_use_driver(val)
        if val:
            status_label.config(text="当前状态：已开启内核驱动", fg="green")
        else:
            status_label.config(text="当前状态：已关闭内核驱动，回退到普通模式", fg="red")

    def run_as_admin(cmd):
        ctypes.windll.shell32.ShellExecuteW(
            None,
            "runas",
            "cmd.exe",
            f"/c {cmd} & pause",
            None,
            1
        )

    def enable_test_mode():
        sys_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "rust_driver_demo.sys").replace("\\", "/")
        ps_content = f"""bcdedit /set testsigning on
$cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject "CN=PyAutoGUIDriverTest" -CertStoreLocation "Cert:\\LocalMachine\\My"
$certPath = "$env:TEMP\\PyAutoGUIDriverTest.cer"
Export-Certificate -Cert $cert -FilePath $certPath
Import-Certificate -FilePath $certPath -CertStoreLocation "Cert:\\LocalMachine\\Root"
Import-Certificate -FilePath $certPath -CertStoreLocation "Cert:\\LocalMachine\\TrustedPublisher"
Set-AuthenticodeSignature -FilePath '{sys_path}' -Certificate $cert
Remove-Item $certPath"""
        import tempfile
        script_path = os.path.join(tempfile.gettempdir(), "sign_driver.ps1")
        with open(script_path, "w", encoding="utf-8-sig") as f:
            f.write(ps_content)
        run_as_admin(f"powershell -ExecutionPolicy Bypass -File \"{script_path}\"")

    def start_service():
        sys_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "rust_driver_demo.sys")
        cmd = f'sc create MyDriver binPath= "{sys_path}" type= kernel & sc start MyDriver'
        run_as_admin(cmd)

    root = tk.Tk()
    root.title("PyAutoGUI 驱动控制")
    root.geometry("450x300")

    var = tk.BooleanVar(value=True)

    status_label = tk.Label(root, text="当前状态：已开启内核驱动", fg="green", font=("Arial", 12))
    status_label.pack(pady=15)

    cb = tk.Checkbutton(root, text="使用内核驱动 (Ring 0)", variable=var, command=on_toggle, font=("Arial", 12))
    cb.pack(pady=10)

    btn_test = tk.Button(root, text="1. 开启系统测试签名模式 (开启后需重启电脑)", command=enable_test_mode, font=("Arial", 10))
    btn_test.pack(pady=5, fill=tk.X, padx=30)

    btn_srv = tk.Button(root, text="2. 创建并启动驱动服务 (在测试模式下运行)", command=start_service, font=("Arial", 10))
    btn_srv.pack(pady=5, fill=tk.X, padx=30)

    on_toggle()
    root.mainloop()
