import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { QueryProvider } from './providers/QueryProvider';
import './styles.css';

const enableMsw = import.meta.env.DEV && import.meta.env.VITE_ENABLE_MSW === 'true';

async function prepare() {
  if (enableMsw) {
    const { worker } = await import('./mocks/browser');
    await worker.start({ onUnhandledRequest: 'bypass' });
  }
}

prepare().then(() => {
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <QueryProvider>
        <App />
      </QueryProvider>
    </React.StrictMode>,
  );
});
