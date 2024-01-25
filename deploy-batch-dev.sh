#!/bin/bash
set -euo pipefail

export AWS_PROFILE=opxs-dev

case $1 in
"batch-email-send")
    ;;
"batch-email-send-feedback")
    ;;
*)
    echo "Usage: $0 <batch-email-send|batch-email-send-feedback>"
    exit 1
    ;;
esac

./build-batch.sh

AWS_REGION="us-east-1"
AWS_ACCOUNT_ID="464209738056"
LAMBDA_ARN="arn:aws:lambda:${AWS_REGION}:${AWS_ACCOUNT_ID}:opxs-$1-lambda"
DOCKER_IMAGE="${AWS_ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com/opxs-$1-lambda-ecr:latest"

aws ecr get-login-password --region ${AWS_REGION} | docker login --username AWS --password-stdin "${AWS_ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com"

if ! docker buildx build --platform linux/amd64 -f "./Dockerfile.run.$1" -t "${DOCKER_IMAGE}" --force-rm=true .; then
    echo "Failed to build docker image"
    exit 1
fi

docker push "${DOCKER_IMAGE}"

aws lambda --region ${AWS_REGION} update-function-code --function-name opxs-$1-lambda --image-uri ${DOCKER_IMAGE}
