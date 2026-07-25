import React, { useRef } from "react";
import { Button } from "@/components/ui/button";
import { Upload } from "lucide-react";

interface LoadTreeDialogProps {
  onLoad: (fileContent: string, fileName: string) => void;
}

export const LoadTreeDialog: React.FC<LoadTreeDialogProps> = ({ onLoad }) => {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleClick = () => {
    fileInputRef.current?.click();
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const result = event.target?.result;
      if (typeof result === "string") {
        onLoad(result, file.name);
      }
    };
    reader.readAsText(file);

    // Reset input so the same file could be loaded again
    e.target.value = "";
  };

  return (
    <div>
      <input
        type="file"
        ref={fileInputRef}
        className="hidden"
        accept=".json"
        onChange={handleFileChange}
      />
      <Button onClick={handleClick} variant="default">
        <Upload className="mr-2 h-4 w-4" />
        Load Tree
      </Button>
    </div>
  );
};
