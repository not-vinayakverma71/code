import axios, { AxiosInstance } from 'axios';

export class ApiService {
    private client: AxiosInstance;
    
    constructor(baseURL: string) {
        this.client = axios.create({
            baseURL,
            timeout: 10000,
        });
        
        this.setupInterceptors();
    }
    
    private setupInterceptors(): void {
        this.client.interceptors.request.use(
            async (config) => {
                const token = localStorage.getItem('token');
                if (token) {
                    config.headers.Authorization = `Bearer ${token}`;
                }
                return config;
            }
        );
    }
    
    async get<T>(url: string): Promise<T> {
        const response = await this.client.get<T>(url);
        return response.data;
    }
}