import { EmailStatus } from "../../types";

export const ALL_EMAIL_STATUSES: EmailStatus[] = [
  "accepted",
  "delivered",
  "failed",
  "held",
  "processing",
  "reattempt",
  "rejected",
];

export const STATUS_SERIES: { name: EmailStatus; color: string }[] = [
  { name: "accepted", color: "gray.6" },
  { name: "delivered", color: "teal.6" },
  { name: "failed", color: "red.6" },
  { name: "held", color: "orange.6" },
  { name: "processing", color: "blue.6" },
  { name: "reattempt", color: "yellow.6" },
  { name: "rejected", color: "grape.6" },
];
