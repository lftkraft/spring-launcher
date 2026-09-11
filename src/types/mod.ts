export interface ModrinthProject {
    id: String;
    slug: String;
    title: String;
    description: String;
    body?: String;
    icon_url?: String;
    categories: String[];
    display_categories?: String[];
    downloads: number;
    followers: number;
    client_side: String;
    server_side: String;
    project_type: String;
}

export interface ModrinthVersion {
    id: String;
    project_id: String;
    name: String;
    version_number: String;
    changelog?: String;
    dependencies: ModrinthDependency[];
    game_versions: String[];
    loaders: String[];
    files: ModrinthFile[];
    date_published: String;
}

export interface ModrinthDependency {
    version_id?: String;
    project_id?: String;
    dependency_type: "required" | "optional" | "incompatible" | "embedded";
}

export interface ModrinthFile {
    url: String;
    filename: String;
    primary: boolean;
    size: number;
    hashes: {
        sha1: String;
        sha512: String;
    };
}

export interface ModrinthSearchHit {
    project_id: string;
    title: string;
    description: string;
    icon_url?: string;
    author: string;
    downloads: number;
    project_type: string;
    client_side: string;
    server_side: string;
    categories?: string[];
    display_categories?: string[];
}

export interface DependencyInfo {
    dependency_type: 'required' | 'optional' | 'incompatible' | 'embedded';
    project: {
        id: string;
        title: string;
        icon_url?: string;
    };
}

