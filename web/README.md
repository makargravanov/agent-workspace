# Frontend modes

Environment flags for local frontend development:

```env
VITE_API_BASE_URL=/api/v1
VITE_ENABLE_MSW=false
VITE_ENABLE_DEV_LOGIN=false
VITE_DEV_API_PROXY_TARGET=http://127.0.0.1:18080
```

## Real backend mode

- `VITE_ENABLE_MSW=false`
- frontend uses the live API configured by `VITE_API_BASE_URL`
- in local Vite development, `/api/v1/*` is proxied to `VITE_DEV_API_PROXY_TARGET`
- `VITE_ENABLE_DEV_LOGIN=true` may be enabled locally if the backend dev login route should be visible

## Mock mode

- `VITE_ENABLE_MSW=true`
- MSW intercepts frontend requests with handlers that match the current backend contract
- useful for isolated UI work when the backend is unavailable
