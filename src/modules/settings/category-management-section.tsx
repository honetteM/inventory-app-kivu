import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import { FolderTree, LoaderCircle } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface BusinessOption {
  id: string;
  name: string;
}

interface Category {
  id: string;
  business_id: string;
  parent_id: string | null;
  name: string;
  description: string | null;
  is_active: boolean;
}

function CategoryTree({
  categories,
  parentId,
  depth = 0,
}: {
  categories: Category[];
  parentId: string | null;
  depth?: number;
}) {
  const children = categories
    .filter((category) => category.parent_id === parentId)
    .sort((left, right) => left.name.localeCompare(right.name));

  if (children.length === 0) {
    return null;
  }

  return (
    <ul className={depth > 0 ? "ml-6 border-l pl-4" : "divide-y"}>
      {children.map((category) => (
        <li key={category.id} className="py-3 first:pt-0 last:pb-0">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-3">
              <span className="flex size-9 items-center justify-center rounded-md bg-muted">
                <FolderTree className="size-4 text-muted-foreground" />
              </span>
              <div>
                <p className="font-medium">{category.name}</p>
                {category.description ? (
                  <p className="text-sm text-muted-foreground">{category.description}</p>
                ) : null}
              </div>
            </div>
            <Badge variant={category.is_active ? "secondary" : "outline"}>
              {category.is_active ? "Active" : "Inactive"}
            </Badge>
          </div>
          <CategoryTree categories={categories} parentId={category.id} depth={depth + 1} />
        </li>
      ))}
    </ul>
  );
}

export function CategoryManagementSection({ businesses }: { businesses: BusinessOption[] }) {
  const [businessId, setBusinessId] = useState("");
  const [categories, setCategories] = useState<Category[]>([]);
  const [parentId, setParentId] = useState("");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!businesses.some((business) => business.id === businessId)) {
      setBusinessId(businesses[0]?.id ?? "");
    }
  }, [businessId, businesses]);

  useEffect(() => {
    if (!businessId) {
      setCategories([]);
      setParentId("");
      return;
    }

    let active = true;
    setIsLoading(true);
    setError(null);

    invoke<Category[]>("category_list", { businessId })
      .then((result) => {
        if (active) {
          setCategories(result);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load categories. ${cause instanceof Error ? cause.message : String(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, [businessId]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!businessId) {
      return;
    }

    setError(null);
    setIsSaving(true);
    try {
      const category = await invoke<Category>("category_create", {
        businessId,
        parentId: parentId || null,
        name: name.trim(),
        description: description.trim() || null,
      });
      setCategories((current) => [...current, category]);
      setName("");
      setDescription("");
      setParentId("");
    } catch (cause) {
      setError(`Could not save the category. ${cause instanceof Error ? cause.message : String(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Product categories</CardTitle>
        <CardDescription>
          Organize the catalogue into business-specific categories and subcategories.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {businesses.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            Register a business before managing its categories.
          </p>
        ) : (
          <>
            <div className="max-w-sm space-y-2">
              <Label htmlFor="category-business">Business</Label>
              <select
                id="category-business"
                value={businessId}
                onChange={(event) => {
                  setBusinessId(event.target.value);
                  setParentId("");
                }}
                className="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
              >
                {businesses.map((business) => (
                  <option key={business.id} value={business.id}>
                    {business.name}
                  </option>
                ))}
              </select>
            </div>

            <form className="grid gap-4 rounded-lg border p-4 md:grid-cols-2" onSubmit={handleSubmit}>
              <div className="space-y-2">
                <Label htmlFor="category-name">Category name</Label>
                <Input
                  id="category-name"
                  maxLength={120}
                  required
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                  placeholder="e.g. Building materials"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="category-parent">Parent category (optional)</Label>
                <select
                  id="category-parent"
                  value={parentId}
                  onChange={(event) => setParentId(event.target.value)}
                  className="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
                >
                  <option value="">No parent (top-level)</option>
                  {categories.map((category) => (
                    <option key={category.id} value={category.id}>
                      {category.name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="space-y-2 md:col-span-2">
                <Label htmlFor="category-description">Description (optional)</Label>
                <Input
                  id="category-description"
                  maxLength={500}
                  value={description}
                  onChange={(event) => setDescription(event.target.value)}
                  placeholder="What belongs in this category?"
                />
              </div>
              <div className="flex justify-end md:col-span-2">
                <Button type="submit" disabled={isSaving || !name.trim()}>
                  {isSaving ? <LoaderCircle className="animate-spin" /> : <FolderTree />}
                  {isSaving ? "Saving..." : "Add category"}
                </Button>
              </div>
            </form>

            {error ? (
              <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
                {error}
              </div>
            ) : null}

            {isLoading ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <LoaderCircle className="h-4 w-4 animate-spin" />
                Loading categories...
              </div>
            ) : categories.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No categories have been created for this business.
              </p>
            ) : (
              <CategoryTree categories={categories} parentId={null} />
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
