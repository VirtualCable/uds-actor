# Base image: Windows Server Core
FROM mcr.microsoft.com/windows/servercore:ltsc2022
SHELL ["powershell","-NoProfile","-Command"]

# Install Chocolatey for easier dependency management
RUN Set-ExecutionPolicy Bypass -Scope Process -Force; \
    [System.Net.ServicePointManager]::SecurityProtocol = 'Tls12'; \
    iex ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1'))

RUN choco install -y cmake --installargs 'ADD_CMAKE_TO_PATH=System' ; \
    choco install -y ninja ; \
    choco install -y llvm ;

    
# Visual Studio Build Tools 2022
RUN choco install -y visualstudio2022buildtools \
    --package-parameters "'--installPath C:\BuildTools --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --quiet --wait --norestart'"

# Define environment variables for Rust, vcpkg, and LLVM. Also define root for FreeRDP build installation
RUN $old=[Environment]::GetEnvironmentVariable('PATH','Machine'); \
    $new=$old+';C:\rust\cargo\bin;C:\vcpkg;C:\Program Files\LLVM\bin'; \
    [Environment]::SetEnvironmentVariable('PATH',$new,'Machine'); \
    [Environment]::SetEnvironmentVariable('CARGO_HOME','C:\rust\cargo','Machine'); \
    [Environment]::SetEnvironmentVariable('RUSTUP_HOME','C:\rust\rustup','Machine'); \
    [Environment]::SetEnvironmentVariable('LIBCLANG_PATH','C:\Program Files\LLVM\bin','Machine');

# install Rust
RUN Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile rustup-init.exe; \
    Start-Process -FilePath .\rustup-init.exe -ArgumentList "'-y --default-toolchain stable-x86_64-pc-windows-msvc'" -Wait; \
    Remove-Item rustup-init.exe;
