import { MantineSize, ThemeIcon, Tooltip } from "@mantine/core";
import { IconInfoCircle } from "@tabler/icons-react";

interface InfoTooltipProps {
  text: string;
  size?: MantineSize | number;
}

export default function InfoTooltip({ text, size }: InfoTooltipProps) {
  return (
    <Tooltip label={text}>
      <ThemeIcon variant="transparent" c="dimmed" size={size ?? "sm"}>
        <IconInfoCircle />
      </ThemeIcon>
    </Tooltip>
  );
}
