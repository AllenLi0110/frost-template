import { NextRequest, NextResponse } from "next/server";

type RouteContext = {
  params: Promise<{
    path?: string[];
  }>;
};

export async function GET(request: NextRequest, context: RouteContext) {
  return proxyCoordinatorRequest(request, context);
}

export async function POST(request: NextRequest, context: RouteContext) {
  return proxyCoordinatorRequest(request, context);
}

async function proxyCoordinatorRequest(
  request: NextRequest,
  context: RouteContext,
) {
  const { path = [] } = await context.params;
  const coordinatorUrl = coordinatorApiUrl(path);
  const body = request.method === "GET" ? undefined : await request.arrayBuffer();
  const response = await fetch(coordinatorUrl, {
    method: request.method,
    headers: forwardedHeaders(request),
    body: body && body.byteLength > 0 ? body : undefined,
    cache: "no-store",
  });

  return new NextResponse(response.body, {
    status: response.status,
    headers: responseHeaders(response),
  });
}

function coordinatorApiUrl(path: string[]): string {
  const baseUrl = coordinatorBaseUrl();
  const url = new URL(`/${path.map(encodeURIComponent).join("/")}`, baseUrl);

  return url.toString();
}

function coordinatorBaseUrl(): string {
  return (
    process.env.COORDINATOR_INTERNAL_URL ??
    process.env.NEXT_PUBLIC_COORDINATOR_URL ??
    "http://localhost:8080"
  );
}

function forwardedHeaders(request: NextRequest): Headers {
  const headers = new Headers();
  const contentType = request.headers.get("content-type");

  if (contentType) {
    headers.set("content-type", contentType);
  }

  return headers;
}

function responseHeaders(response: Response): Headers {
  const headers = new Headers();
  const contentType = response.headers.get("content-type");

  if (contentType) {
    headers.set("content-type", contentType);
  }

  return headers;
}
