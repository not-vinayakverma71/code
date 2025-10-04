// Service 665
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service665 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}