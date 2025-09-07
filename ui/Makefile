PROTO_OUT_DIR=src/proto/
GRPC_WEB_PLUGIN := tools/protoc-gen-grpc-web
JS_PLUGIN := tools/protoc-gen-js
GRPC_WEB_OUTPUT_OPTIONS := import_style=typescript,mode=grpcwebtext
PROTOC_FLAGS := -I../proto --plugin=protoc-gen-grpc-web=$(GRPC_WEB_PLUGIN) --plugin=protoc-gen-js=$(JS_PLUGIN)
SERVICES := todolist user accounting
SERVICE_CLIENT_PBS := $(foreach service,$(SERVICES),src/proto/$(shell echo $(service)|sed 's/.*/\u&/')ServiceClientPb.ts)

all: frontend

.PHONY: frontend proto
frontend: proto src/assets/favicon.ico
	npm run build

proto: $(SERVICES:%=src/proto/%_pb.d.ts) $(SERVICES:%=src/proto/%_pb.js) $(SERVICE_CLIENT_PBS)

tools/:
	@mkdir -p $@

tools/protoc-gen-js-downloaded/:
	@mkdir -p $@

$(PROTO_OUT_DIR):
	@mkdir -p $@

src/proto/%_pb.js src/proto/%_pb.d.ts: ../proto/%.proto $(GRPC_WEB_PLUGIN) $(JS_PLUGIN) |$(PROTO_OUT_DIR)
	protoc $(PROTOC_FLAGS) --js_out=import_style=commonjs:src/proto --grpc-web_out=$(GRPC_WEB_OUTPUT_OPTIONS):src/proto $<

.SECONDEXPANSION: $(SERVICE_CLIENT_PBS)

$(SERVICE_CLIENT_PBS): src/proto/%ServiceClientPb.ts: src/proto/$$(shell echo $$*|tr A-Z a-z)_pb.js

src/assets/favicon.ico: src/assets/logo.png
	ffmpeg -i $< -vf scale="64:-1"  $@

.DELETE_ON_ERROR:
$(GRPC_WEB_PLUGIN): |tools/
	curl -sSL https://github.com/grpc/grpc-web/releases/download/1.5.0/protoc-gen-grpc-web-1.5.0-linux-x86_64 -o $@
	cd tools/ && echo "2e6e074497b221045a14d5a54e9fc910945bfdd1198b12b9fc23686a95671d64 protoc-gen-grpc-web" | sha256sum -c
	chmod +x $@

$(JS_PLUGIN): |tools/ tools/protoc-gen-js-downloaded/
	curl -sSL https://github.com/protocolbuffers/protobuf-javascript/releases/download/v3.21.4/protobuf-javascript-3.21.4-linux-x86_64.zip -o tools/protobuf-javascript-3.21.4-linux-x86_64.zip
	cd tools/protoc-gen-js-downloaded/ && unzip ../protobuf-javascript-3.21.4-linux-x86_64.zip && mv bin/protoc-gen-js ../protoc-gen-js
	cd tools/ && rm -rf protobuf-javascript-3.21.4-linux-x86_64.zip protoc-gen-js-downloaded/
	chmod +x $@
