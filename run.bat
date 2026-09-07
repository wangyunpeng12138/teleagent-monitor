@echo off
chcp 65001 >nul 2>&1
title TeleAgent Monitor - 一键启动

echo ====================================
echo   TeleAgent Monitor 一键启动脚本
echo ====================================
echo.

:: 项目根目录
set PROJECT_DIR=%~dp0
cd /d "%PROJECT_DIR%"

:: 1. 初始化 MSVC C++ 编译环境
echo [1/4] 初始化 MSVC 环境...
set VCVARS="C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if exist %VCVARS% (
    call %VCVARS% >nul 2>&1
    echo       MSVC 环境初始化成功
) else (
    echo [警告] 未找到 vcvars64.bat，跳过 MSVC 初始化
    echo        如遇编译报错请手动初始化 MSVC 环境
)

:: 2. 设置 Rust 环境变量
echo [2/4] 设置 Rust 环境变量...
set RUSTUP_HOME=D:\app\rust\rustup
set CARGO_HOME=D:\app\rust\cargo
set PATH=D:\app\rust\cargo\bin;%PATH%
echo       Rust 环境变量已设置

:: 3. 构建前端
echo [3/4] 构建前端...
call npm run build >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [警告] 前端构建失败，尝试使用已有的 dist 目录
) else (
    echo       前端构建成功
)

:: 4. 启动 Tauri 应用
echo [4/4] 启动 TeleAgent Monitor...
echo.
echo 窗口即将弹出，如未出现请检查任务栏。
echo 关闭此窗口不会退出程序，需在任务管理器中结束 teleagent-monitor.exe
echo.

npx tauri dev

:: 如果 tauri dev 失败，尝试直接运行已编译的 exe
if %ERRORLEVEL% neq 0 (
    echo.
    echo [提示] tauri dev 启动失败，尝试直接运行已编译的 exe...
    set EXE_PATH=%PROJECT_DIR%src-tauri\target\debug\teleagent-monitor.exe
    if exist "%EXE_PATH%" (
        start "" "%EXE_PATH%"
    ) else (
        echo [错误] 未找到已编译的 exe，请检查编译是否完成
        pause
    )
)

echo.
echo TeleAgent Monitor 已退出。
pause
