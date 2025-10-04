// Service 1425
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service1425 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}