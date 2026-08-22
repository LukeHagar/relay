module sub-latency

go 1.23

require github.com/relay-webhook/relay-sdk-go v0.0.0

require github.com/gorilla/websocket v1.5.3 // indirect

replace github.com/relay-webhook/relay-sdk-go => ../../../sdks/go
