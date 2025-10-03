// Service 1883
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service1883 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}