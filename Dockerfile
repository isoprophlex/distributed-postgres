# Use the official ubuntu:22.04 image as a parent image
FROM --platform=$BUILDPLATFORM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get upgrade -y && \
    apt-get install -y curl git build-essential libicu-dev flex bison libreadline-dev zlib1g-dev libpq-dev

# Install rust inside the container
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Copy the files in the machine to the Docker image
COPY ./ ./
# Configure the PATH for Rust
ENV PATH=/root/.cargo/bin:$PATH

# Clone the repository
RUN git clone -b dockerfile --single-branch https://github.com/isoprophlex/distributed-postgres.git /app

# Change to the working directory
WORKDIR /app

# Create a non-root user
RUN useradd -m pguser

# Change the owner of the app directory to the non-root user
RUN chown -R pguser:pguser /app

# Change to the non-root user
USER pguser

# Give execute permissions to the script
RUN chmod +x ./init-and-start-sv.sh