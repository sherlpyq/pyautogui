import sys
import os
import tkinter as tk
from tkinter import messagebox

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pyautogui

def on_toggle():
    val = var.get()
    pyautogui.set_use_driver(val)
    if val:
        status_label.config(text="当前状态：已开启内核驱动", fg="green")
    else:
        status_label.config(text="当前状态：已关闭内核驱动，回退到普通模式", fg="red")

root = tk.Tk()
root.title("PyAutoGUI 驱动控制")
root.geometry("400x200")

var = tk.BooleanVar(value=True)

status_label = tk.Label(root, text="当前状态：已开启内核驱动", fg="green", font=("Arial", 12))
status_label.pack(pady=20)

cb = tk.Checkbutton(root, text="使用内核驱动 (Ring 0)", variable=var, command=on_toggle, font=("Arial", 12))
cb.pack(pady=10)

on_toggle()

root.mainloop()
