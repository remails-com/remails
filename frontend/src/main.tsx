import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "@mantine/core/styles.css";
import { createTheme, MantineProvider } from "@mantine/core";
import "@mantine/notifications/styles.css";
import { Notifications } from "@mantine/notifications";
import { ModalsProvider } from "@mantine/modals";

const element = document.getElementById("root")!;
const root = createRoot(element);

const theme = createTheme({
  primaryColor: "remails-red",
  colors: {
    "remails-red": [
      "#ffb7a7",
      "#ffa592",
      "#ff937c",
      "#ff8166",
      "#ff6f50",
      "#ff5d3a",
      "#ff4b24",
      "#cc3c1d",
      "#992d16",
      "#661e0e",
    ],
  },
});

root.render(
  <StrictMode>
    <MantineProvider theme={theme}>
      <ModalsProvider>
        <Notifications />
        <App />
      </ModalsProvider>
    </MantineProvider>
  </StrictMode>
);
