FROM rust:1.82

WORKDIR /usr/src/highres-service
COPY . .

RUN cargo build --release

CMD ["./target/release/highres-service"]
