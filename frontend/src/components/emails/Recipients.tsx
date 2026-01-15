import { Badge, MantineSpacing, StyleProp, Tooltip } from "@mantine/core";
import { DeliveryStatus, Log, EmailMetadata } from "../../types";
import { formatDateTime } from "../../util.ts";
import { useDisclosure } from "@mantine/hooks";
import { ReactElement, useState } from "react";
import { IconCheck, IconClock, IconX } from "@tabler/icons-react";
import { ConnectionLog } from "./ConnectionLog.tsx";

interface RecipientsProps {
  email: EmailMetadata;
  ml?: StyleProp<MantineSpacing>;
  mr?: StyleProp<MantineSpacing>;
}

const deliveryStatus: {
  [key in DeliveryStatus["type"]]: { color: string; icon?: ReactElement };
} = {
  NotSent: { color: "secondary", icon: undefined },
  Success: { color: "green", icon: <IconCheck size={16} /> },
  Reattempt: { color: "orange", icon: <IconClock size={16} /> },
  Failed: { color: "red", icon: <IconX size={16} /> },
};

export function Recipients({ email, mr, ml }: RecipientsProps): ReactElement {
  const [opened, { open, close }] = useDisclosure(false);
  const [log, setLog] = useState<Log>({ lines: [] });

  const badges = email.recipients.map((recipient: string) => {
    const details = email.delivery_details[recipient];
    const status = details?.status ?? { type: "NotSent" };

    let tooltip = "Email not (yet) sent";
    if (status.type == "Failed") {
      tooltip = "Permanent failure";
    } else if (status.type == "Reattempt") {
      tooltip = "Temporary failure";
    } else if (status.type == "Success") {
      tooltip = `Delivered on ${formatDateTime(status.delivered)}`;
    }

    return (
      <Tooltip label={tooltip} key={recipient}>
        <Badge
          style={{ cursor: details?.log ? "pointer" : "default" }}
          color={deliveryStatus[status.type].color}
          variant="light"
          ml={ml}
          mr={mr}
          rightSection={deliveryStatus[status.type].icon}
          tt="none"
          size="lg"
          onClick={(e) => {
            e.stopPropagation();
            if (details?.log) {
              setLog(details.log);
              open();
            }
          }}
        >
          {recipient}
        </Badge>
      </Tooltip>
    );
  });

  return (
    <>
      <ConnectionLog log={log} opened={opened} close={close} />
      {badges}
    </>
  );
}
