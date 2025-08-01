import { Button, Group, Modal, Text, Title } from "@mantine/core";
import { CreatedInvite } from "../../types";
import { CopyableCode } from "../CopyableCode";
import { formatDateTime } from "../../util";

interface NewInviteProps {
  opened: boolean;
  close: () => void;
  invite: CreatedInvite | null;
}

export default function NewInvite({ opened, close, invite }: NewInviteProps) {
  return (
    <Modal
      opened={opened}
      onClose={close}
      title={
        <Title order={3} component="span">
          Created new invite link
        </Title>
      }
      size="lg"
      padding="xl"
    >
      Share this link with the person you want to add to this organization:
      <CopyableCode props={{ my: "xs" }}>
        {`${window.location.protocol}//${window.location.host}/invite/${invite?.organization_id}/${invite?.id}/${invite?.password}`}
      </CopyableCode>
      <Text size="sm" c="dimmed">
        This link is valid until {invite ? formatDateTime(invite.expires_at) : "..."} and can only be used once.
      </Text>
      <Group mt="md" justify="flex-end">
        <Button onClick={close}>Done</Button>
      </Group>
    </Modal>
  );
}
