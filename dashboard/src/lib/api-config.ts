export function getApiBaseUrl(): string {
  if (typeof window !== 'undefined') {
    return process.env.NEXT_PUBLIC_GATEWAY_URL || 'http://localhost:8000';
  }
  return process.env.NEXT_PUBLIC_GATEWAY_URL || process.env.GATEWAY_URL || 'http://localhost:8000';
}

