import { createRoot } from "react-dom/client";
import App from "./App";
import "@mantine/core/styles.css";
import "@mantine/dates/styles.css";
import "@mantine/nprogress/styles.css";
import { createTheme, MantineProvider } from "@mantine/core";
import "@mantine/notifications/styles.css";
import { Notifications } from "@mantine/notifications";
import { ModalsProvider } from "@mantine/modals";

const element = document.getElementById("root")!;
const root = createRoot(element);

const theme = createTheme({
  primaryColor: "remails-red",
  // https://mantine.dev/colors-generator/?color=FF4B24
  colors: {
    "remails-red": [
      '#ffebe4',
      '#ffd6cc',
      '#ffac9a',
      '#ff7f64',
      '#ff5936',
      '#ff4b24',
      '#ff3407',
      '#e42500',
      '#cc1e00',
      '#b21100',
    ],
  },
});

root.render(
  <MantineProvider theme={theme}>
    <ModalsProvider>
      <Notifications />
      <App />
    </ModalsProvider>
  </MantineProvider>
);
