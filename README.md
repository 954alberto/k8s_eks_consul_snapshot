# consnap

## Introduction

This tool has been created to work in the same namespace where the official hashicorp consul helm chart is deployed. It is intended to run as a cronjob on eks and upload the snapshot to an S3 bucket. To do so it depends on a k8s service account with an annotation to assume an IAM role and this role needs a policy with enough privileges on the given bucket.
The tool is written in Rust and uses rusoto to assume the role and get temporary AWS credentials to PutObject to S3.

An example of a yaml manifest to deploy it can be found below:

```yaml
apiVersion: batch/v1beta1
kind: CronJob
metadata:
  name: consnap
  namespace: consul
spec:
  schedule: "0 */2 * * *" # At minute zero every two hours
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: consnap
            image: sbpcat/consnap:0.1.1
            imagePullPolicy: Always
            args:
            - /bin/sh
            - -c
            - /home/consnap/consnap
            env:
              - name: S3_BUCKET_NAME
                value: "consul-snapshot"
              - name: CONSUL_HTTP_ADDR
                value: "http://consul-consul-ui/v1/snapshot"
              - name: CONSUL_HTTP_TOKEN
                valueFrom:
                  secretKeyRef:
                    name: consul-consul-bootstrap-acl-token
                    key: token
          restartPolicy: Never
          securityContext:
            fsGroup: 1000
            runAsGroup: 1000
            runAsNonRoot: true
            runAsUser: 1000
          serviceAccount: consul-consul-server
          serviceAccountName: consul-consul-server
```

## Requirements

The tool needs to find in the container the environment variables:

```bash
S3_BUCKET_NAME="consul-snapshot"
CONSUL_HTTP_ADDR="http://consul-consul-ui/v1/snapshot"
CONSUL_HTTP_TOKEN="<sometoken>"
```

Please note that CONSUL_HTTP_ADDR needs the path to the snapshot endpoint.

everytime that the tool is executed it will upload a tar.gz to S3 like `snapshot_2020_08_19_14_59_38.tar.gz`

## How to build the binary

```bash
docker run --rm --user "0":"0" -v "$PWD":/usr/src/myapp -w /usr/src/myapp rust:1.45.2 cargo build --release --target-dir=linux
```

This will download the official docker image debian based of the rust compiler, it will start a local container and compile the binary. Once it is done the you can find the binary at `linux/release/consnap`.

If you are on Mac OS X remmeber that Docker does not run native but on a vm so compiling time might take a bit longer.

## How to build the docker image

After the new binary is created copy it to the docker folder where the Dockerfile is and execute:

```bash
docker build . -t sbpcat/consnap:<sometag>
```

Once your new imageas is correctly tagged push it to the registry

```bash
docker push sbpcat/consnap:<sometag>
```
