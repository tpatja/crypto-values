#!/usr/bin/env bash

set -eu -o pipefail

TMPFILE=$(mktemp)

cleanup() {
    rm -f "$TMPFILE"
}

trap cleanup SIGINT SIGTERM ERR EXIT

export AWS_PAGER=""
FUNCTION_NAME="crypto-values"
ROLE_NAME="crypto-values-role"

retry() {
    local -r -i max_attempts="$1"; shift
    local -i attempt_num=1
    until "$@"
    do
        if ((attempt_num==max_attempts))
        then
            return 1
        else
            echo "Attempt $attempt_num failed, trying again in $attempt_num seconds..."
            sleep $((attempt_num++))
        fi
    done
}

destroy_aws_resources() {
    aws lambda delete-function --function-name ${FUNCTION_NAME} || true
    aws events remove-targets --rule ${FUNCTION_NAME}_scheduled \
      --ids ${FUNCTION_NAME}_scheduled-lambda || true
    aws events delete-rule --name ${FUNCTION_NAME}_scheduled || true
    aws iam detach-role-policy --role-name ${ROLE_NAME} \
      --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole || true
    aws iam delete-role --role-name ${ROLE_NAME} || true
}


create_aws_resources() {

  cat <<EOF > "$TMPFILE"
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": {
      "Service": [
          "events.amazonaws.com",
          "lambda.amazonaws.com"
      ]
    },
    "Action": "sts:AssumeRole"
  }]
}
EOF

  aws iam create-role --role-name ${ROLE_NAME} \
    --assume-role-policy-document "file://$TMPFILE"
  aws iam attach-role-policy --role-name ${ROLE_NAME} \
    --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole

  ROLE_ARN=$(aws iam get-role --role-name ${ROLE_NAME} \
    | jq .Role.Arn | sed 's/"//g')

  # the role takes some time to get propagated, so we retry max 5 times
  # (this does not work: aws iam wait role-exists --role-name crypto-values-role)
  retry 5 aws lambda create-function --function-name ${FUNCTION_NAME} \
	  --runtime provided.al2 \
		--role "${ROLE_ARN}" \
		--handler bootstrap \
		--environment "{\"Variables\": $(cat .env.aws.json)}" \
		--timeout 600 \
		--zip-file fileb://target/lambda/lambda-handler/bootstrap.zip

  aws events put-rule --name ${FUNCTION_NAME}_scheduled \
    --role-arn "${ROLE_ARN}" \
    --schedule-expression 'rate(5 minutes)' --state ENABLED

  RULE_ARN=$(aws events describe-rule --name ${FUNCTION_NAME}_scheduled \
    | jq .Arn | sed 's/"//g')
  FN_ARN=$(aws lambda get-function-configuration --function-name ${FUNCTION_NAME} \
    | jq .FunctionArn)

  cat <<EOF > "$TMPFILE"
{
    "Rule": "${FUNCTION_NAME}_scheduled",
    "Targets": [{
      "Id": "${FUNCTION_NAME}_scheduled-lambda",
      "Arn": $FN_ARN,
      "Input": "{}"
    }]
}
EOF

  aws events put-targets --cli-input-json "file://$TMPFILE"
  aws lambda add-permission --function-name ${FUNCTION_NAME} \
    --principal events.amazonaws.com --source-arn "$RULE_ARN" \
    --action 'lambda:invokeFunction' --statement-id ${FUNCTION_NAME}-permissions

}

if [ $# -eq 0 ]; then
  echo "Usage: $0 [-d] [-c]"
  exit 1
fi

# : as first char in optstring -> take control of flags that aren't in the list I set
# no : after flag -> flag doesn't take an argument
#  https://ss64.com/osx/getopts.html
while getopts ":dc" opt; do
  case ${opt} in
    d)
      destroy_aws_resources
      ;;
    c)
      create_aws_resources
      ;;
    \?)
      echo "Invalid option: $OPTARG" >&2
      ;;
  esac
done
