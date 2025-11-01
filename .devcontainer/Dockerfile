FROM mcr.microsoft.com/devcontainers/rust

COPY .devcontainer/setup.sh /tmp/setup.sh
RUN chmod +x /tmp/setup.sh
USER vscode
RUN /tmp/setup.sh