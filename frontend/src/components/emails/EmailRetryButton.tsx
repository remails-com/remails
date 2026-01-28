import { notifications } from "@mantine/notifications";
import { Email, EmailMetadata } from "../../types.ts";
import { IconReload } from "@tabler/icons-react";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { is_in_the_future } from "../../util.ts";
import { errorNotification } from "../../notify.tsx";
import { MaintainerActionIcon, MaintainerButton } from "../RoleButtons.tsx";
import { useState } from "react";

export default function EmailRetryButton({
  email,
  updateEmail,
  small,
}: {
  email: EmailMetadata;
  updateEmail: (email_id: string, update: Partial<Email>) => void;
  small?: boolean;
}) {
  const { currentOrganization } = useOrganizations();
  const [loading, setLoading] = useState(false);

  if (!currentOrganization) {
    return null;
  }

  const email_endpoint = `/api/organizations/${currentOrganization.id}/emails/${email.id}`;

  async function retry() {
    const res = await fetch(`${email_endpoint}/retry`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
    });
    if (res.status !== 200) {
      errorNotification("Email could not be retried");
      console.error(res);
      return;
    }

    notifications.show({
      title: "Scheduled retry",
      message: "Email will be retried soon",
      color: "blue",
      autoClose: 20000,
      icon: <IconReload size={20} />,
    });

    await new Promise((r) => setTimeout(r, 2000));

    const update = await fetch(email_endpoint);
    if (update.status !== 200) {
      errorNotification("Email could not be found");
      console.error(update);
      return;
    }
    updateEmail(email.id, await update.json());
  }

  const onClick = () => {
    setLoading(true);
    retry().finally(() => setLoading(false));
  };

  const status_retryable = !(email.status == "processing" || email.status == "accepted" || email.status == "delivered");
  const already_scheduled = email.retry_after && !is_in_the_future(email.retry_after);

  const can_retry = status_retryable && !already_scheduled;

  const tooltip = status_retryable
    ? already_scheduled
      ? "Email is already scheduled to retry as soon as possible"
      : "(Re-)schedule retry"
    : `Email is ${email.status}`;

  if (small) {
    return (
      <MaintainerActionIcon
        tooltip={tooltip}
        disabled={!can_retry}
        onClick={onClick}
        variant="light"
        size={30}
        loading={loading}
      >
        <IconReload />
      </MaintainerActionIcon>
    );
  } else {
    return (
      <MaintainerButton
        tooltip={tooltip}
        leftSection={<IconReload />}
        disabled={!can_retry}
        onClick={onClick}
        loading={loading}
      >
        Retry
      </MaintainerButton>
    );
  }
}
