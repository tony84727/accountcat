# Rsbuild project

## Setup

Install the dependencies:

```bash
npm install
```

### Generate proto files

This project uses gRPC-Web. Before building the UI you need
`protoc`, `protoc-gen-js` and `protoc-gen-grpc_web` available in your `PATH`.
After installing them run:

```bash
make proto
```

to generate the TypeScript files under `src/proto`.

## Get started

Start the dev server, and the app will be available at [http://localhost:3000](http://localhost:3000).

```bash
npm dev
```

Build the app for production:

```bash
npm build
```

Preview the production build locally:

```bash
npm preview
```

## Learn more

To learn more about Rsbuild, check out the following resources:

- [Rsbuild documentation](https://rsbuild.rs) - explore Rsbuild features and APIs.
- [Rsbuild GitHub repository](https://github.com/web-infra-dev/rsbuild) - your feedback and contributions are welcome!
