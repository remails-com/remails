import {StrictMode} from 'react'
import {createRoot} from 'react-dom/client'
import App from './App'
import '@mantine/core/styles.css';
import {createTheme, MantineProvider} from '@mantine/core';
import '@mantine/notifications/styles.css';
import {Notifications} from '@mantine/notifications';
import {ModalsProvider} from "@mantine/modals";

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
      <ModalsProvider>
        <Notifications/>
        <App/>
      </ModalsProvider>
    </MantineProvider>
  </StrictMode>,
);
