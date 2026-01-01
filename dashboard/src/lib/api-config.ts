export function getApiBaseUrl(): string {
  if (typeof window !== 'undefined') {
    return process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';
  }
  return process.env.GATEWAY_INTERNAL_URL || process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';
}

