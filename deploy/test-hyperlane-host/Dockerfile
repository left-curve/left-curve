FROM debian:bookworm-slim

# Install SSH server, Python (necessario per Ansible) e Docker CLI (solo client, non daemon)
RUN apt-get update && \
    apt-get install -y openssh-server sudo curl ca-certificates gnupg lsb-release python3 python3-pip && \
    # Install Docker CLI (compatibile con Docker del host)
    install -m 0755 -d /etc/apt/keyrings && \
    curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg && \
    chmod a+r /etc/apt/keyrings/docker.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null && \
    apt-get update && \
    apt-get install -y docker-ce-cli && \
    rm -rf /var/lib/apt/lists/*

# Setup SSH
RUN mkdir /var/run/sshd && \
    echo 'root:test' | chpasswd && \
    sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/' /etc/ssh/sshd_config && \
    sed -i 's/#PasswordAuthentication yes/PasswordAuthentication yes/' /etc/ssh/sshd_config

# Creare utente debian (come negli altri host)
RUN useradd -m -s /bin/bash debian && \
    echo 'debian:test' | chpasswd && \
    echo 'debian ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers

EXPOSE 22

CMD ["/usr/sbin/sshd", "-D"]

