FROM scratch
COPY ./target/x86_64-unknown-linux-musl/release/foo /usr/bin/
EXPOSE 8000
ENTRYPOINT ["foo"]
