// EXACT 1:1 TRANSLATION FROM TYPESCRIPT MCP Server Creation Prompts
// Critical: NO CHANGES to message formats - must match TypeScript exactly

use anyhow::Result;

/// Create MCP Server Instructions - EXACT translation from TypeScript
pub async fn create_mcp_server_instructions(
    mcp_servers_path: Option<String>,
) -> Result<String> {
    let servers_path = mcp_servers_path
        .unwrap_or_else(|| "~/.config/mcp/servers".to_string());
    
    // EXACT text from TypeScript create-mcp-server.ts
    Ok(format!(r#"You have the ability to create an MCP server and add it to a configuration file that will then expose the tools and resources for you to use with `use_mcp_tool` and `access_mcp_resource`.

When creating MCP servers, it's important to understand that they operate in a non-interactive environment. The server cannot initiate OAuth flows, open browser windows, or prompt for user input during runtime. All credentials and authentication tokens must be provided upfront through environment variables in the MCP settings configuration. For example, Spotify's API uses OAuth to get a refresh token for the user, but the MCP server cannot initiate this flow. While you can walk the user through obtaining an application client ID and secret, you may have to create a separate one-time setup script (like get-refresh-token.js) that captures and logs the final piece of the puzzle: the user's refresh token (i.e. you might run the script using execute_command which would open a browser for authentication, and then log the refresh token so that you can see it in the command output for you to use in the MCP settings configuration).

Unless the user specifies otherwise, new local MCP servers should be created in: {}

### MCP Server Types and Configuration

MCP servers can be configured in two ways in the MCP settings file:

1. Local (Stdio) Server Configuration:
```json
{{
	"mcpServers": {{
		"local-weather": {{
			"command": "node",
			"args": ["/path/to/weather-server/build/index.js"],
			"env": {{
				"OPENWEATHER_API_KEY": "your-api-key"
			}}
		}}
	}}
}}
```

2. Remote (SSE) Server Configuration:
```json
{{
	"mcpServers": {{
		"remote-weather": {{
			"url": "https://api.example.com/mcp",
			"headers": {{
				"Authorization": "Bearer your-api-key"
			}}
		}}
	}}
}}
```

Common configuration options for both types:
- `disabled`: (optional) Set to true to temporarily disable the server
- `timeout`: (optional) Maximum time in seconds to wait for server responses (default: 60)
- `alwaysAllow`: (optional) Array of tool names that don't require user confirmation
- `disabledTools`: (optional) Array of tool names that are not included in the system prompt and won't be used

### Example Local MCP Server

For example, if the user wanted to give you the ability to retrieve weather information, you could create an MCP server that uses the OpenWeather API to get weather information, add it to the MCP settings configuration file, and then notice that you now have access to new tools and resources in the system prompt that you might use to show the user your new capabilities.

The following example demonstrates how to build a local MCP server that provides weather data functionality using the Stdio transport. While this example shows how to implement resources, resource templates, and tools, in practice you should prefer using tools since they are more flexible and can handle dynamic parameters. The resource and resource template implementations are included here mainly for demonstration purposes of the different MCP capabilities, but a real weather server would likely just expose tools for fetching weather data. (The following steps are for macOS)

1. Use the `create-typescript-server` tool to bootstrap a new project in the default MCP servers directory:

```bash
cd {}
npx @modelcontextprotocol/create-server weather-server
cd weather-server
# Install dependencies
npm install axios zod @modelcontextprotocol/sdk
```

This will create a new project with the following structure:

```
weather-server/
	├── package.json
			{{
				...
				"type": "module", // added by default, uses ES module syntax (import/export) rather than CommonJS (require/module.exports) (Important to know if you create additional scripts in this server repository like a get-refresh-token.js script)
				"scripts": {{
					"build": "tsc && node -e \"require('fs').chmodSync('build/index.js', '755')\"",
					...
				}}
				...
			}}
	├── tsconfig.json
	└── src/
			└── index.ts      # Main server implementation
```

2. Replace `src/index.ts` with the following:

```typescript
#!/usr/bin/env node
import {{ McpServer, ResourceTemplate }} from "@modelcontextprotocol/sdk/server/mcp.js";
import {{ StdioServerTransport }} from "@modelcontextprotocol/sdk/server/stdio.js";
import {{ z }} from "zod";
import axios from 'axios';

const API_KEY = process.env.OPENWEATHER_API_KEY; // provided by MCP config
if (!API_KEY) {{
  throw new Error('OPENWEATHER_API_KEY environment variable is required');
}}

// Define types for OpenWeather API responses
interface WeatherData {{
  main: {{
    temp: number;
    humidity: number;
  }};
  weather: Array<{{
    description: string;
  }}>;
  wind: {{
    speed: number;
  }};
}}

interface ForecastData {{
  list: Array<WeatherData & {{
    dt_txt: string;
  }}>;
}}

// Create an MCP server
const server = new McpServer({{
  name: "weather-server",
  version: "0.1.0"
}});

// Create axios instance for OpenWeather API
const weatherApi = axios.create({{
  baseURL: 'http://api.openweathermap.org/data/2.5',
  params: {{
    appid: API_KEY,
    units: 'metric',
  }},
}});

// Add a tool for getting weather forecasts
server.tool(
  "get_forecast",
  {{
    city: z.string().describe("City name"),
    days: z.number().min(1).max(5).optional().describe("Number of days (1-5)"),
  }},
  async ({{ city, days = 3 }}) => {{
    try {{
      const response = await weatherApi.get<ForecastData>('forecast', {{
        params: {{
          q: city,
          cnt: Math.min(days, 5) * 8,
        }},
      }});

      return {{
        content: [
          {{
            type: "text",
            text: JSON.stringify(response.data.list, null, 2),
          }},
        ],
      }};
    }} catch (error) {{
      if (axios.isAxiosError(error)) {{
        return {{
          content: [
            {{
              type: "text",
              text: `Weather API error: ${{
                error.response?.data.message ?? error.message
              }}`,
            }},
          ],
          isError: true,
        }};
      }}
      throw error;
    }}
  }}
);

// Add a resource for current weather in San Francisco
server.resource(
  "sf_weather",
  {{ uri: "weather://San Francisco/current", list: true }},
  async (uri) => {{
    try {{
      const response = weatherApi.get<WeatherData>('weather', {{
        params: {{ q: "San Francisco" }},
      }});

      return {{
        contents: [
          {{
            uri: uri.href,
            mimeType: "application/json",
            text: JSON.stringify(
              {{
                temperature: response.data.main.temp,
                conditions: response.data.weather[0].description,
                humidity: response.data.main.humidity,
                windSpeed: response.data.wind.speed,
              }},
              null,
              2
            ),
          }},
        ],
      }};
    }} catch (error) {{
      if (axios.isAxiosError(error)) {{
        return {{
          contents: [
            {{
              uri: uri.href,
              mimeType: "text/plain",
              text: `Weather API error: ${{
                error.response?.data.message ?? error.message
              }}`,
            }},
          ],
        }};
      }}
      throw error;
    }}
  }}
);

// Start the server using Stdio transport
async function main() {{
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error('Weather MCP server running');
}}

main().catch((error) => {{
  console.error('Server error:', error);
  process.exit(1);
}});
```

3. Build and test the server:

```bash
npm run build
```

4. Add the server to your MCP settings:

```json
{{
  "mcpServers": {{
    "weather-server": {{
      "command": "node",
      "args": ["{}"],
      "env": {{
        "OPENWEATHER_API_KEY": "your_api_key_here"
      }}
    }}
  }}
}}
```

5. Reload the configuration and verify the tools and resources are available through use_mcp_tool and access_mcp_resource

### Important Notes

- MCP servers are completely independent processes that communicate with the assistant through a standardized protocol
- Authentication must be configured upfront via environment variables or config files
- The server cannot open browsers or interact with users during runtime
- All tools should include proper error handling for API failures
- Use descriptive names and clear documentation for tools and resources"#,
        servers_path, servers_path, format!("{}/weather-server/build/index.js", servers_path)
    ))
}
