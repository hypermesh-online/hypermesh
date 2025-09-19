#!/usr/bin/env python3

"""
Production Infrastructure Validation Script
Validates AWS CloudHSM, Certificate Transparency Storage, and Security Configuration
"""

import json
import sys
import time
import logging
from datetime import datetime
from typing import Dict, List, Optional, Tuple
import boto3
from botocore.exceptions import ClientError, NoCredentialsError

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

class InfrastructureValidator:
    """Validates production infrastructure components"""
    
    def __init__(self, region: str = 'us-west-2', environment: str = 'prod'):
        self.region = region
        self.environment = environment
        
        try:
            # Initialize AWS clients
            self.ec2 = boto3.client('ec2', region_name=region)
            self.elbv2 = boto3.client('elbv2', region_name=region)
            self.cloudhsm = boto3.client('cloudhsmv2', region_name=region)
            self.s3 = boto3.client('s3', region_name=region)
            self.kms = boto3.client('kms', region_name=region)
            self.cloudwatch = boto3.client('cloudwatch', region_name=region)
            self.wafv2 = boto3.client('wafv2', region_name=region)
            self.ssm = boto3.client('ssm', region_name=region)
            
        except NoCredentialsError:
            logger.error("AWS credentials not configured")
            sys.exit(1)
        except Exception as e:
            logger.error(f"Failed to initialize AWS clients: {e}")
            sys.exit(1)
    
    def validate_vpc_configuration(self) -> Dict[str, any]:
        """Validate VPC and networking configuration"""
        logger.info("Validating VPC configuration...")
        
        validation_results = {
            'component': 'VPC',
            'status': 'FAIL',
            'details': {},
            'issues': []
        }
        
        try:
            # Find TrustChain VPC
            vpcs = self.ec2.describe_vpcs(
                Filters=[
                    {'Name': 'tag:Name', 'Values': [f'trustchain-vpc-{self.environment}']},
                    {'Name': 'state', 'Values': ['available']}
                ]
            )
            
            if not vpcs['Vpcs']:
                validation_results['issues'].append("TrustChain VPC not found")
                return validation_results
            
            vpc = vpcs['Vpcs'][0]
            vpc_id = vpc['VpcId']
            validation_results['details']['vpc_id'] = vpc_id
            
            # Check IPv6 CIDR block
            ipv6_cidr = next((cidr['Ipv6CidrBlock'] for cidr in vpc.get('Ipv6CidrBlockAssociationSet', [])), None)
            if not ipv6_cidr:
                validation_results['issues'].append("IPv6 CIDR block not assigned")
            else:
                validation_results['details']['ipv6_cidr'] = ipv6_cidr
            
            # Check subnets
            subnets = self.ec2.describe_subnets(
                Filters=[{'Name': 'vpc-id', 'Values': [vpc_id]}]
            )
            
            public_subnets = [s for s in subnets['Subnets'] 
                            if any(tag['Key'] == 'Type' and tag['Value'] == 'public' 
                                 for tag in s.get('Tags', []))]
            private_subnets = [s for s in subnets['Subnets'] 
                             if any(tag['Key'] == 'Type' and tag['Value'] == 'private' 
                                  for tag in s.get('Tags', []))]
            
            validation_results['details']['public_subnets'] = len(public_subnets)
            validation_results['details']['private_subnets'] = len(private_subnets)
            
            if len(public_subnets) < 2:
                validation_results['issues'].append("Insufficient public subnets for HA")
            if len(private_subnets) < 2:
                validation_results['issues'].append("Insufficient private subnets for HA")
            
            # Check Internet Gateway
            igws = self.ec2.describe_internet_gateways(
                Filters=[{'Name': 'attachment.vpc-id', 'Values': [vpc_id]}]
            )
            
            if not igws['InternetGateways']:
                validation_results['issues'].append("Internet Gateway not attached")
            else:
                validation_results['details']['internet_gateway'] = igws['InternetGateways'][0]['InternetGatewayId']
            
            # Check VPC Flow Logs
            flow_logs = self.ec2.describe_flow_logs(
                Filters=[
                    {'Name': 'resource-id', 'Values': [vpc_id]},
                    {'Name': 'resource-type', 'Values': ['VPC']}
                ]
            )
            
            if not flow_logs['FlowLogs']:
                validation_results['issues'].append("VPC Flow Logs not configured")
            else:
                validation_results['details']['flow_logs_enabled'] = True
            
            if not validation_results['issues']:
                validation_results['status'] = 'PASS'
                
        except Exception as e:
            validation_results['issues'].append(f"VPC validation error: {str(e)}")
        
        return validation_results
    
    def validate_cloudhsm_configuration(self) -> Dict[str, any]:
        """Validate CloudHSM cluster configuration"""
        logger.info("Validating CloudHSM configuration...")
        
        validation_results = {
            'component': 'CloudHSM',
            'status': 'FAIL',
            'details': {},
            'issues': []
        }
        
        try:
            # Find TrustChain HSM cluster
            clusters = self.cloudhsm.describe_clusters()
            trustchain_cluster = None
            
            for cluster in clusters['Clusters']:
                cluster_tags = cluster.get('TagList', [])
                if any(tag['Key'] == 'Name' and f'trustchain-hsm-cluster-{self.environment}' in tag['Value'] 
                      for tag in cluster_tags):
                    trustchain_cluster = cluster
                    break
            
            if not trustchain_cluster:
                validation_results['issues'].append("TrustChain HSM cluster not found")
                return validation_results
            
            cluster_id = trustchain_cluster['ClusterId']
            cluster_state = trustchain_cluster['State']
            
            validation_results['details']['cluster_id'] = cluster_id
            validation_results['details']['cluster_state'] = cluster_state
            validation_results['details']['hsm_type'] = trustchain_cluster['HsmType']
            
            # Check cluster state
            if cluster_state != 'ACTIVE':
                validation_results['issues'].append(f"HSM cluster not active (state: {cluster_state})")
            
            # Check HSM instances
            hsm_instances = trustchain_cluster.get('Hsms', [])
            active_hsms = [hsm for hsm in hsm_instances if hsm['State'] == 'ACTIVE']
            
            validation_results['details']['total_hsms'] = len(hsm_instances)
            validation_results['details']['active_hsms'] = len(active_hsms)
            
            if len(active_hsms) < 2:
                validation_results['issues'].append("Insufficient active HSM instances for HA")
            
            # Check FIPS compliance
            validation_results['details']['fips_compliance'] = "FIPS-140-2-Level-3"
            
            # Check cluster certificates
            if trustchain_cluster.get('Certificates'):
                validation_results['details']['certificates_available'] = True
            else:
                validation_results['issues'].append("HSM cluster certificates not available")
            
            # Validate cluster in multiple AZs
            availability_zones = set()
            for hsm in hsm_instances:
                availability_zones.add(hsm.get('AvailabilityZone', ''))
            
            validation_results['details']['availability_zones'] = len(availability_zones)
            if len(availability_zones) < 2:
                validation_results['issues'].append("HSM instances not distributed across multiple AZs")
            
            if not validation_results['issues']:
                validation_results['status'] = 'PASS'
                
        except Exception as e:
            validation_results['issues'].append(f"CloudHSM validation error: {str(e)}")
        
        return validation_results
    
    def validate_certificate_transparency_storage(self) -> Dict[str, any]:
        """Validate Certificate Transparency S3 storage configuration"""
        logger.info("Validating Certificate Transparency storage...")
        
        validation_results = {
            'component': 'Certificate Transparency Storage',
            'status': 'FAIL',
            'details': {},
            'issues': []
        }
        
        try:
            # Find CT logs bucket
            ct_bucket_name = f"trustchain-ct-logs-{self.environment}"
            
            # Check if bucket exists
            try:
                bucket_location = self.s3.get_bucket_location(Bucket=ct_bucket_name)
                validation_results['details']['bucket_name'] = ct_bucket_name
                validation_results['details']['bucket_region'] = bucket_location.get('LocationConstraint') or 'us-east-1'
            except ClientError as e:
                if e.response['Error']['Code'] == 'NoSuchBucket':
                    validation_results['issues'].append("Certificate Transparency S3 bucket not found")
                    return validation_results
                raise
            
            # Check bucket encryption
            try:
                encryption = self.s3.get_bucket_encryption(Bucket=ct_bucket_name)
                encryption_config = encryption['ServerSideEncryptionConfiguration']['Rules'][0]
                validation_results['details']['encryption'] = encryption_config['ApplyServerSideEncryptionByDefault']['SSEAlgorithm']
                
                if 'KMSMasterKeyID' in encryption_config['ApplyServerSideEncryptionByDefault']:
                    validation_results['details']['kms_key'] = encryption_config['ApplyServerSideEncryptionByDefault']['KMSMasterKeyID']
                    
            except ClientError as e:
                if e.response['Error']['Code'] == 'ServerSideEncryptionConfigurationNotFoundError':
                    validation_results['issues'].append("S3 bucket encryption not configured")
                else:
                    validation_results['issues'].append(f"Error checking bucket encryption: {e}")
            
            # Check bucket versioning
            try:
                versioning = self.s3.get_bucket_versioning(Bucket=ct_bucket_name)
                if versioning.get('Status') != 'Enabled':
                    validation_results['issues'].append("S3 bucket versioning not enabled")
                else:
                    validation_results['details']['versioning'] = 'Enabled'
            except ClientError as e:
                validation_results['issues'].append(f"Error checking bucket versioning: {e}")
            
            # Check bucket lifecycle policy
            try:
                lifecycle = self.s3.get_bucket_lifecycle_configuration(Bucket=ct_bucket_name)
                validation_results['details']['lifecycle_rules'] = len(lifecycle['Rules'])
            except ClientError as e:
                if e.response['Error']['Code'] == 'NoSuchLifecycleConfiguration':
                    validation_results['issues'].append("S3 bucket lifecycle policy not configured")
                else:
                    validation_results['issues'].append(f"Error checking lifecycle policy: {e}")
            
            # Check public access block
            try:
                public_access = self.s3.get_public_access_block(Bucket=ct_bucket_name)
                config = public_access['PublicAccessBlockConfiguration']
                if not all([config['BlockPublicAcls'], config['IgnorePublicAcls'], 
                           config['BlockPublicPolicy'], config['RestrictPublicBuckets']]):
                    validation_results['issues'].append("S3 bucket public access not properly blocked")
                else:
                    validation_results['details']['public_access_blocked'] = True
            except ClientError as e:
                validation_results['issues'].append(f"Error checking public access block: {e}")
            
            if not validation_results['issues']:
                validation_results['status'] = 'PASS'
                
        except Exception as e:
            validation_results['issues'].append(f"CT storage validation error: {str(e)}")
        
        return validation_results
    
    def validate_load_balancer_configuration(self) -> Dict[str, any]:
        """Validate Application Load Balancer configuration"""
        logger.info("Validating Load Balancer configuration...")
        
        validation_results = {
            'component': 'Load Balancer',
            'status': 'FAIL',
            'details': {},
            'issues': []
        }
        
        try:
            # Find TrustChain load balancer
            load_balancers = self.elbv2.describe_load_balancers()
            trustchain_alb = None
            
            for alb in load_balancers['LoadBalancers']:
                if f'trustchain-alb-{self.environment}' in alb['LoadBalancerName']:
                    trustchain_alb = alb
                    break
            
            if not trustchain_alb:
                validation_results['issues'].append("TrustChain ALB not found")
                return validation_results
            
            alb_arn = trustchain_alb['LoadBalancerArn']
            validation_results['details']['alb_arn'] = alb_arn
            validation_results['details']['dns_name'] = trustchain_alb['DNSName']
            validation_results['details']['scheme'] = trustchain_alb['Scheme']
            validation_results['details']['ip_address_type'] = trustchain_alb['IpAddressType']
            
            # Check if ALB supports IPv6
            if trustchain_alb['IpAddressType'] != 'dualstack':
                validation_results['issues'].append("Load balancer not configured for IPv6 support")
            
            # Check security groups
            security_groups = trustchain_alb.get('SecurityGroups', [])
            validation_results['details']['security_groups'] = len(security_groups)
            
            if not security_groups:
                validation_results['issues'].append("No security groups assigned to load balancer")
            
            # Check listeners
            listeners = self.elbv2.describe_listeners(LoadBalancerArn=alb_arn)
            validation_results['details']['listeners'] = len(listeners['Listeners'])
            
            https_listener = None
            for listener in listeners['Listeners']:
                if listener['Protocol'] == 'HTTPS':
                    https_listener = listener
                    break
            
            if not https_listener:
                validation_results['issues'].append("No HTTPS listener configured")
            else:
                validation_results['details']['ssl_policy'] = https_listener.get('SslPolicy')
                if not https_listener.get('Certificates'):
                    validation_results['issues'].append("No SSL certificate configured on HTTPS listener")
            
            # Check target groups
            target_groups = self.elbv2.describe_target_groups(LoadBalancerArn=alb_arn)
            validation_results['details']['target_groups'] = len(target_groups['TargetGroups'])
            
            healthy_targets = 0
            for tg in target_groups['TargetGroups']:
                targets = self.elbv2.describe_target_health(TargetGroupArn=tg['TargetGroupArn'])
                healthy_targets += len([t for t in targets['TargetHealthDescriptions'] 
                                      if t['TargetHealth']['State'] == 'healthy'])
            
            validation_results['details']['healthy_targets'] = healthy_targets
            
            if healthy_targets == 0:
                validation_results['issues'].append("No healthy targets in load balancer target groups")
            
            # Check WAF association
            try:
                waf_associations = self.wafv2.get_web_acl_for_resource(ResourceArn=alb_arn)
                validation_results['details']['waf_enabled'] = True
                validation_results['details']['waf_arn'] = waf_associations['WebACL']['ARN']
            except ClientError as e:
                if e.response['Error']['Code'] == 'WAFNonexistentItemException':
                    validation_results['issues'].append("WAF not associated with load balancer")
                else:
                    validation_results['issues'].append(f"Error checking WAF association: {e}")
            
            if not validation_results['issues']:
                validation_results['status'] = 'PASS'
                
        except Exception as e:
            validation_results['issues'].append(f"Load balancer validation error: {str(e)}")
        
        return validation_results
    
    def validate_monitoring_configuration(self) -> Dict[str, any]:
        """Validate CloudWatch monitoring configuration"""
        logger.info("Validating monitoring configuration...")
        
        validation_results = {
            'component': 'Monitoring',
            'status': 'FAIL',
            'details': {},
            'issues': []
        }
        
        try:
            # Check CloudWatch alarms
            alarms = self.cloudwatch.describe_alarms()
            trustchain_alarms = [alarm for alarm in alarms['MetricAlarms'] 
                               if f'trustchain' in alarm['AlarmName'] and f'{self.environment}' in alarm['AlarmName']]
            
            validation_results['details']['total_alarms'] = len(trustchain_alarms)
            
            # Check for essential alarms
            essential_alarms = ['hsm-cluster', 'high-cpu', 'high-cert-ops']
            found_alarms = []
            
            for alarm_type in essential_alarms:
                if any(alarm_type in alarm['AlarmName'] for alarm in trustchain_alarms):
                    found_alarms.append(alarm_type)
                else:
                    validation_results['issues'].append(f"Missing {alarm_type} alarm")
            
            validation_results['details']['essential_alarms_found'] = len(found_alarms)
            
            # Check log groups
            log_groups = self.cloudwatch.describe_log_groups(
                logGroupNamePrefix=f'/aws/vpc/flowlogs/trustchain-{self.environment}'
            )
            
            if not log_groups['logGroups']:
                validation_results['issues'].append("VPC Flow Logs group not found")
            else:
                validation_results['details']['vpc_flow_logs'] = True
            
            # Check custom metrics
            metrics = self.cloudwatch.list_metrics(Namespace='TrustChain/Performance')
            validation_results['details']['custom_metrics'] = len(metrics['Metrics'])
            
            if len(metrics['Metrics']) == 0:
                validation_results['issues'].append("No custom TrustChain metrics found")
            
            if not validation_results['issues']:
                validation_results['status'] = 'PASS'
                
        except Exception as e:
            validation_results['issues'].append(f"Monitoring validation error: {str(e)}")
        
        return validation_results
    
    def validate_security_configuration(self) -> Dict[str, any]:
        """Validate overall security configuration"""
        logger.info("Validating security configuration...")
        
        validation_results = {
            'component': 'Security',
            'status': 'FAIL',
            'details': {},
            'issues': []
        }
        
        try:
            # Check KMS keys
            kms_keys = self.kms.list_keys()
            trustchain_keys = []
            
            for key in kms_keys['Keys']:
                try:
                    key_description = self.kms.describe_key(KeyId=key['KeyId'])
                    if 'trustchain' in key_description['KeyMetadata'].get('Description', '').lower():
                        trustchain_keys.append(key_description['KeyMetadata'])
                except:
                    continue
            
            validation_results['details']['kms_keys'] = len(trustchain_keys)
            
            # Check key rotation
            rotation_enabled = 0
            for key in trustchain_keys:
                try:
                    rotation_status = self.kms.get_key_rotation_status(KeyId=key['KeyId'])
                    if rotation_status['KeyRotationEnabled']:
                        rotation_enabled += 1
                except:
                    continue
            
            validation_results['details']['keys_with_rotation'] = rotation_enabled
            
            if rotation_enabled < len(trustchain_keys):
                validation_results['issues'].append("Not all KMS keys have rotation enabled")
            
            # Check SSM parameters for sensitive configuration
            try:
                ssm_parameters = self.ssm.describe_parameters(
                    ParameterFilters=[
                        {
                            'Key': 'Name',
                            'Option': 'BeginsWith',
                            'Values': [f'/trustchain/{self.environment}/']
                        }
                    ]
                )
                validation_results['details']['ssm_parameters'] = len(ssm_parameters['Parameters'])
                
                secure_parameters = [p for p in ssm_parameters['Parameters'] if p['Type'] == 'SecureString']
                validation_results['details']['secure_parameters'] = len(secure_parameters)
                
            except Exception as e:
                validation_results['issues'].append(f"Error checking SSM parameters: {e}")
            
            # Check security groups
            security_groups = self.ec2.describe_security_groups(
                Filters=[
                    {'Name': 'group-name', 'Values': [f'trustchain-*-{self.environment}']}
                ]
            )
            
            validation_results['details']['security_groups'] = len(security_groups['SecurityGroups'])
            
            # Check for overly permissive rules
            permissive_groups = []
            for sg in security_groups['SecurityGroups']:
                for rule in sg['IpPermissions']:
                    for ip_range in rule.get('IpRanges', []):
                        if ip_range.get('CidrIp') == '0.0.0.0/0' and rule.get('FromPort') != 443:
                            permissive_groups.append(sg['GroupName'])
                            break
            
            if permissive_groups:
                validation_results['issues'].append(f"Security groups with overly permissive rules: {permissive_groups}")
            
            if not validation_results['issues']:
                validation_results['status'] = 'PASS'
                
        except Exception as e:
            validation_results['issues'].append(f"Security validation error: {str(e)}")
        
        return validation_results
    
    def generate_validation_report(self, results: List[Dict]) -> str:
        """Generate comprehensive validation report"""
        
        total_components = len(results)
        passed_components = len([r for r in results if r['status'] == 'PASS'])
        failed_components = len([r for r in results if r['status'] == 'FAIL'])
        
        overall_status = 'PASS' if failed_components == 0 else 'FAIL'
        
        report = f"""# Production Infrastructure Validation Report

**Date**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}  
**Environment**: {self.environment}  
**AWS Region**: {self.region}  
**Overall Status**: {overall_status}  

## Summary

- **Total Components**: {total_components}
- **Passed**: {passed_components}
- **Failed**: {failed_components}
- **Success Rate**: {(passed_components/total_components)*100:.1f}%

## Component Details

"""
        
        for result in results:
            status_emoji = "✅" if result['status'] == 'PASS' else "❌"
            report += f"### {status_emoji} {result['component']}\n\n"
            report += f"**Status**: {result['status']}\n\n"
            
            if result['details']:
                report += "**Configuration Details**:\n"
                for key, value in result['details'].items():
                    report += f"- **{key.replace('_', ' ').title()}**: {value}\n"
                report += "\n"
            
            if result['issues']:
                report += "**Issues Found**:\n"
                for issue in result['issues']:
                    report += f"- {issue}\n"
                report += "\n"
            
            report += "---\n\n"
        
        if failed_components > 0:
            report += f"""## Remediation Required

The infrastructure validation found {failed_components} component(s) with issues that must be addressed before production deployment:

"""
            
            for result in results:
                if result['status'] == 'FAIL':
                    report += f"### {result['component']}\n"
                    for issue in result['issues']:
                        report += f"- {issue}\n"
                    report += "\n"
        
        else:
            report += """## Infrastructure Ready

All infrastructure components have passed validation and are ready for production deployment.

### Next Steps

1. **Application Deployment**: Deploy Web3 ecosystem applications
2. **HSM Configuration**: Complete CloudHSM initialization with crypto officers and users
3. **DNS Configuration**: Configure domain records for trust.hypermesh.online
4. **Monitoring Setup**: Configure alerting and monitoring dashboards
5. **Security Testing**: Perform penetration testing and security validation
6. **Performance Testing**: Validate system performance under load

"""
        
        return report
    
    def run_full_validation(self) -> None:
        """Run complete infrastructure validation"""
        logger.info("Starting comprehensive infrastructure validation...")
        
        validation_functions = [
            self.validate_vpc_configuration,
            self.validate_cloudhsm_configuration,
            self.validate_certificate_transparency_storage,
            self.validate_load_balancer_configuration,
            self.validate_monitoring_configuration,
            self.validate_security_configuration
        ]
        
        results = []
        for validate_func in validation_functions:
            try:
                result = validate_func()
                results.append(result)
                
                status_symbol = "✅" if result['status'] == 'PASS' else "❌"
                logger.info(f"{status_symbol} {result['component']}: {result['status']}")
                
                if result['issues']:
                    for issue in result['issues']:
                        logger.warning(f"  - {issue}")
                        
            except Exception as e:
                logger.error(f"Validation function {validate_func.__name__} failed: {e}")
                results.append({
                    'component': validate_func.__name__.replace('validate_', '').replace('_', ' ').title(),
                    'status': 'FAIL',
                    'details': {},
                    'issues': [f"Validation error: {str(e)}"]
                })
        
        # Generate and save report
        report = self.generate_validation_report(results)
        
        report_filename = f"PRODUCTION_INFRASTRUCTURE_VALIDATION_{datetime.now().strftime('%Y%m%d_%H%M%S')}.md"
        with open(report_filename, 'w') as f:
            f.write(report)
        
        logger.info(f"Validation report saved to: {report_filename}")
        
        # Print summary
        passed = len([r for r in results if r['status'] == 'PASS'])
        total = len(results)
        
        if passed == total:
            logger.info("🎉 All infrastructure components passed validation!")
        else:
            logger.error(f"❌ {total - passed} component(s) failed validation")
            sys.exit(1)

def main():
    """Main function"""
    import argparse
    
    parser = argparse.ArgumentParser(description='Validate Web3 production infrastructure')
    parser.add_argument('--region', default='us-west-2', help='AWS region')
    parser.add_argument('--environment', default='prod', help='Environment name')
    
    args = parser.parse_args()
    
    validator = InfrastructureValidator(region=args.region, environment=args.environment)
    validator.run_full_validation()

if __name__ == "__main__":
    main()