import { CircleDashed } from "lucide-react";
import { Card, CardContent, CardDescription, CardTitle } from "@/components/ui/card";
import { PageHeader } from "@/components/shared/page-header";

export function PagePlaceholder({ title, description }: { title: string; description?: string }) {
  return (
    <div className="mx-auto max-w-4xl space-y-6">
      <PageHeader eyebrow="Business" title={title} description={description} />
      <Card>
        <CardContent className="flex items-start gap-4 py-6">
          <div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-accent text-accent-foreground">
            <CircleDashed className="size-5" />
          </div>
          <div className="space-y-1">
            <CardTitle>Module not yet available</CardTitle>
            <CardDescription>
              This area will be implemented in a later development step. Other workspace tools remain available from the navigation.
            </CardDescription>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
