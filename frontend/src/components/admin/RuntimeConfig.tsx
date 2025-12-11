import { useForm } from "@mantine/form";
import { useProjects } from "../../hooks/useProjects.ts";
import { Popover, Select, Stack, TextInput, Text, Group, Button, Switch } from "@mantine/core";
import { IconHelp } from "@tabler/icons-react";
import { useRuntimeConfig } from "../../hooks/useRuntimeConfig.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { notifications } from "@mantine/notifications";
import { errorNotification } from "../../notify.tsx";
import { useOrganizations } from "../../hooks/useOrganizations.ts";

interface FormValues {
  system_email_address: string | null;
  system_email_project: string | null;
  enable_account_creation: boolean;
}

export default function RuntimeConfig() {
  const { projects } = useProjects();
  const { organizations, currentOrganization } = useOrganizations();
  const { dispatch } = useRemails();
  const { runtimeConfig } = useRuntimeConfig();

  const configForm = useForm<FormValues>({
    initialValues: {
      system_email_address: runtimeConfig.system_email_address,
      system_email_project: runtimeConfig.system_email_project,
      enable_account_creation: runtimeConfig.enable_account_creation,
    },
    validate: {
      system_email_address: (value) => (!value || /^\S+@\S+$/.test(value) ? null : "Invalid email"),
    },
  });

  const updateSettings = async (c: FormValues) => {
    const res = await fetch("/api/config/runtime", {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(c),
    });
    if (res.status === 200) {
      dispatch({ type: "set_runtime_config", config: await res.json() });
      configForm.resetDirty();
      notifications.show({
        title: "Updated",
        color: "green",
        message: "",
      });
    } else {
      errorNotification("Config could not be updated");
      console.error(res);
    }
  };

  const systemEmailDropdownOptions = [
    {
      group: currentOrganization?.name || "Unknown organization",
      items: projects.map((p) => {
        return { value: p.id, label: p.name };
      }),
    },
  ];

  if (runtimeConfig.system_email_organization && currentOrganization?.id !== runtimeConfig.system_email_organization) {
    systemEmailDropdownOptions.push({
      group:
        organizations.find((o) => o.id === runtimeConfig.system_email_organization)?.name ||
        `organization ${runtimeConfig.system_email_organization}`,
      items: [{ value: runtimeConfig.system_email_project, label: runtimeConfig.system_email_project_name }],
    });
  }

  return (
    <Stack>
      <form onSubmit={configForm.onSubmit(updateSettings)}>
        <Stack>
          <TextInput
            label={
              <Group gap="xs">
                System email address
                <Popover width={200} position="bottom" withArrow shadow="md">
                  <Popover.Target>
                    <IconHelp size={20} color="gray" />
                  </Popover.Target>
                  <Popover.Dropdown>
                    <Text size="xs">
                      Use this address to send out system emails such as password resets. Make sure the domain is
                      configured in the corresponding organization.
                    </Text>
                  </Popover.Dropdown>
                </Popover>
              </Group>
            }
            key={configForm.key("system_email_address")}
            value={configForm.values.system_email_address || ""}
            placeholder="e.g., noreply@remails.com"
            type="email"
            error={configForm.errors.system_email_address}
            onChange={(event) => {
              const value = event.currentTarget.value.trim() || null;
              configForm.setFieldValue("system_email_address", value);
            }}
          />
          <Select
            label="System email project"
            placeholder="Pick project to send system emails"
            clearable
            searchable
            nothingFoundMessage="This project does not exist, in the currently active organization..."
            data={systemEmailDropdownOptions}
            value={configForm.values.system_email_project}
            onChange={(value) => configForm.setFieldValue("system_email_project", value)}
          />
          <Switch
            checked={configForm.values.enable_account_creation}
            onChange={(ev) => configForm.setFieldValue("enable_account_creation", ev.currentTarget.checked)}
            label="Enable new account creation"
          />
          <Button type="submit" disabled={!configForm.isDirty()}>
            Save
          </Button>
        </Stack>
      </form>
    </Stack>
  );
}
