import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import '@mantine/core/styles.css';
import { createTheme, MantineProvider } from '@mantine/core';

const element = document.getElementById('root')!;
const root = createRoot(element);

const theme = createTheme({
  primaryColor: 'bright-purple',
  colors: {

    'bright-purple': [
      "#f6e9ff",
      "#e6cfff",
      "#ca9cff",
      "#ac65fe",
      "#9337fd",
      "#831bfd",
      "#7c0cfe",
      "#6a00e3",
      "#5d00cb",
      "#5000b3"
    ],
  },
});

root.render(
  <StrictMode>
    <MantineProvider theme={theme}>
      <App />
    </MantineProvider>
  </StrictMode>,
);
