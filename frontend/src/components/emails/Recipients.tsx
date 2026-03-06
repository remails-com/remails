import { Badge, MantineSpacing, StyleProp, Tooltip } from "@mantine/core";
import { DeliveryStatus, EmailMetadata, DeliveryDetails } from "../../types";
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

const DELIVERY_STATUS_STYLES: {
  [key in DeliveryStatus["type"]]: { color: string; icon?: ReactElement };
} = {
  NotSent: { color: "secondary", icon: undefined },
  Success: { color: "green", icon: <IconCheck size={16} /> },
  Reattempt: { color: "orange", icon: <IconClock size={16} /> },
  Failed: { color: "red", icon: <IconX size={16} /> },
};

export function Recipient({ details, recipient, props }: { details: DeliveryDetails, recipient: string, props: React.ComponentProps<typeof Badge> }): ReactElement {
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
        color={DELIVERY_STATUS_STYLES[status.type].color}
        variant="light"
        rightSection={DELIVERY_STATUS_STYLES[status.type].icon}
        tt="none"
        size="lg"
        {...props}
      >
        {recipient}
      </Badge>
    </Tooltip>
  );

}

export function Recipients({ email, mr, ml }: RecipientsProps): ReactElement {
  const [opened, { open, close }] = useDisclosure(false);
  const [log, setLog] = useState<DeliveryDetails>({ log: { lines: [] }, status: { type: "NotSent" } });

  const badges = email.recipients.map((recipient: string) => {
    const details = email.delivery_details[recipient];
    return (
      <Recipient details={details} recipient={recipient} props={{
        ml,
        mr,
        onClick: (e: MouseEvent) => {
          e.stopPropagation();
          if (details?.log) {
            setLog(details);
            open();
          }
        },
        style: { cursor: details?.log ? "pointer" : "default" }
      }} />
    );
  }
  );

  return (
    <>
      <ConnectionLog details={log} recipient={"aaa@test.com"} opened={opened} close={close} />
      {badges}
    </>
  );
}
