FROM rust:latest

RUN cargo install bunyan


# COPY ./hypertube_backend ./hypertube_backend

WORKDIR /hypertube_backend

# npm -- for rqbit

# RUN cargo install --path .

# RUN cargo build
# COPY ./migrations ./migrations

EXPOSE 8000

RUN cargo install sqlx-cli
RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres

# COPY ../script.sh /hypertube_backend/script.sh
# RUN chmod +x /hypertube_backend/script.sh

# RUN sqlx migrate run

CMD ["./script.sh"]
# CMD [ "ls", '-a' ]
# CMD ["tail", "-f", "/dev/null"]