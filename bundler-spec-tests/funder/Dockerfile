FROM ghcr.io/foundry-rs/foundry:latest

RUN apk add jq
ADD funder.sh /
RUN chmod a+rx /funder.sh
ENTRYPOINT /funder.sh 
