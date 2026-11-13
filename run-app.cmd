@echo off
:: Code Sandbox Windows Launcher
:: Launches the compiled Code Sandbox desktop application via WSLg GUI
start "" wsl -d Ubuntu /mnt/c/My_Files/riz_code/target/release/code-sandbox-app
