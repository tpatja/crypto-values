.PHONY: build-aws-lambda
build-aws-lambda: src/*.rs src/bin/*.rs
	cargo lambda build --release --bin lambda-handler --features="aws-lambda" \
		--output-format zip --x86-64

.PHONY: update-aws-lambda
update-aws-lambda:
	aws --no-cli-pager lambda update-function-code --function-name crypto-values \
	  --zip-file fileb://target/lambda/lambda-handler/bootstrap.zip

.PHONY: undeploy-aws-lambda
undeploy-aws-lambda:
	bash ./aws-setup.sh -d

.PHONY: deploy-aws-lambda
deploy-aws-lambda:
	bash ./aws-setup.sh -c

.PHONY: build-cli
build-cli: src/*.rs
	cargo build --bin crypto-values --features="cli" --release
