FROM alpine:latest

RUN apk add --no-cache ca-certificates

RUN mkdir -p /usr/share/doc/oatmeal/copyright/
COPY LICENSE THIRDPARTY.html /usr/share/doc/oatmeal/copyright/

ENV OATMEAL_EDITOR=none
COPY oatmeal /usr/bin/

ENTRYPOINT ["/usr/bin/oatmeal"]
