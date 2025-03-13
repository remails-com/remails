import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import '@mantine/core/styles.css';
import { createTheme, MantineProvider } from '@mantine/core';

const element = document.getElementById('root')!;
const root = createRoot(element);

const theme = createTheme({
  /** Put your mantine theme override here */
});

root.render(
  <StrictMode>
    <MantineProvider theme={theme}>
      <App />
    </MantineProvider>
  </StrictMode>,
);
