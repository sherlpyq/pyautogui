import io
import os
import re
from setuptools import setup
from setuptools_rust import Binding, RustExtension

scriptFolder = os.path.dirname(os.path.realpath(__file__))
os.chdir(scriptFolder)

# Find version info from module (without importing the module):
with open('pyautogui/__init__.py', 'r') as fd:
    version = re.search(r'^__version__\s*=\s*[\'"]([^\'"]*)[\'"]',
                        fd.read(), re.MULTILINE).group(1)

# Use the README.md content for the long description:
with io.open("README.md", encoding="utf-8") as fileObj:
    long_description = fileObj.read()

setup(
    name='PyAutoGUI',
    version=version,
    url='https://github.com/asweigart/pyautogui',
    author='Al Sweigart',
    author_email='al@inventwithpython.com',
    description=('PyAutoGUI lets Python control the mouse and keyboard, and other GUI automation tasks. For Windows, macOS, and Linux, on Python 3 and 2.'),
    long_description=long_description,
    long_description_content_type="text/markdown",
    license='BSD',
    packages=['pyautogui'],
    rust_extensions=[RustExtension("pyautogui._rust_core", binding=Binding.PyO3)],
    zip_safe=False,
    test_suite='tests',
    install_requires=['pyobjc-core;platform_system=="Darwin"',
                      'pyobjc-framework-quartz;platform_system=="Darwin"',
                      'python3-Xlib;platform_system=="Linux" and python_version>="3.0"',
                      'python-xlib;platform_system=="Linux" and python_version<"3.0"',
                      'pymsgbox',
                      'pytweening>=1.0.4',
                      'pyscreeze>=0.1.21',
                      'pygetwindow>=0.0.5',
                      'mouseinfo'],
    keywords="gui automation test testing keyboard mouse cursor click press keystroke control",
    classifiers=[
        'Development Status :: 4 - Beta',
        'Environment :: Win32 (MS Windows)',
        'Environment :: X11 Applications',
        'Environment :: MacOS X',
        'Intended Audience :: Developers',
        'License :: OSI Approved :: BSD License',
        'Operating System :: OS Independent',
        'Programming Language :: Python',
        'Programming Language :: Python :: 3',
        'Programming Language :: Python :: 3.1',
        'Programming Language :: Python :: 3.2',
        'Programming Language :: Python :: 3.3',
        'Programming Language :: Python :: 3.4',
        'Programming Language :: Python :: 3.5',
        'Programming Language :: Python :: 3.6',
        'Programming Language :: Python :: 3.7',
        'Programming Language :: Python :: 3.8',
        'Programming Language :: Python :: 3.9',
        'Programming Language :: Python :: 3.10',
        'Programming Language :: Python :: 3.11',
    ],
    package_data={'pyautogui': ['rust_driver_demo.sys', '_rust_core.pyd']},
    entry_points={
        'console_scripts': [
            'pyautogui = pyautogui._cli:main',
        ],
    },
)
