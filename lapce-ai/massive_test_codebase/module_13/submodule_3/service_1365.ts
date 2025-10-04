// Service 1365
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service1365 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}