PROTO_OUT_DIR=src/proto/
GRPC_WEB_PLUGIN := tools/protoc-gen-grpc-web
GRPC_WEB_OUTPUT_OPTIONS := import_style=typescript,mode=grpcwebtext

all: frontend

.PHONY: frontend proto
frontend: proto
	npm run build

proto: src/proto/UserServiceClientPb.ts src/proto/TodolistServiceClientPb.ts

tools/:
	@mkdir -p tools

$(PROTO_OUT_DIR):
	@mkdir -p $@

PROTOC_FLAGS := -I../proto --plugin=protoc-gen-grpc-web=$(GRPC_WEB_PLUGIN)

src/proto/user_pb.d.ts src/proto/UserServiceClientPb.ts: ../proto/user.proto tools/protoc-gen-grpc-web $(PROTO_OUT_DIR)
	protoc $(PROTOC_FLAGS) --js_out=import_style=commonjs:src/proto --grpc-web_out=$(GRPC_WEB_OUTPUT_OPTIONS):src/proto $<

src/proto/todolist_pb.d.ts src/proto/TodolistServiceClientPb.ts: ../proto/todolist.proto tools/protoc-gen-grpc-web $(PROTO_OUT_DIR)
	protoc $(PROTOC_FLAGS) --js_out=import_style=commonjs:src/proto --grpc-web_out=$(GRPC_WEB_OUTPUT_OPTIONS):src/proto $<


.DELETE_ON_ERROR:
$(GRPC_WEB_PLUGIN): tools/
	curl -sL https://github.com/grpc/grpc-web/releases/download/1.5.0/protoc-gen-grpc-web-1.5.0-linux-x86_64 -o $@
	cd tools/ && echo "2e6e074497b221045a14d5a54e9fc910945bfdd1198b12b9fc23686a95671d64 protoc-gen-grpc-web" | sha256sum -c
	chmod +x $@


